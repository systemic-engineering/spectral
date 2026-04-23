//! encode — markdown ↔ Gestalt<Document>.
//!
//! Two entry points:
//!   - `from_markdown(text)` — parse markdown into Gestalt<Document>
//!   - `to_markdown(gestalt)` — render Gestalt<Document> to markdown
//!
//! Markdown encoding uses pulldown-cmark.
//! Extensions: YAML frontmatter, [[wiki-links]], GFM callouts [!KIND], breath markers (..)

use crate::document::*;
use crate::domain;
use crate::semantic::{CalloutKind, Mark, MarkSet, Meta, Role};

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a markdown string into a domain::Gestalt<Document>.
pub fn from_markdown(markdown: &str) -> domain::Gestalt<domain::Document> {
    let (metadata, body) = extract_frontmatter(markdown);
    let preprocessed = preprocess_wikilinks(body);
    let options = Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(&preprocessed, options);
    let flat_blocks = collect_blocks(parser);
    let nested = nest_sections(flat_blocks);
    lift_gestalt_internal(nested, metadata_to_meta(metadata))
}

/// Parse a .gestalt canonical text into a domain::Gestalt<Document>.
pub fn from_gestalt(input: &str) -> domain::Gestalt<domain::Document> {
    let (metadata, body) = extract_frontmatter(input);
    let preprocessed = preprocess_wikilinks(body);
    let options = Options::ENABLE_STRIKETHROUGH;
    let parser = Parser::new_ext(&preprocessed, options);
    let flat_blocks = collect_blocks(parser);
    let processed = postprocess_gestalt(flat_blocks);
    let nested = nest_sections(processed);
    lift_gestalt_internal(nested, metadata_to_meta(metadata))
}

/// Render a Gestalt<Document> to markdown text.
pub fn to_markdown(g: &domain::Gestalt<domain::Document>) -> String {
    let mut out = String::new();
    // Render frontmatter from head Meta::Extension entries
    let fm: Vec<_> = g
        .head
        .iter()
        .filter_map(|m| match m {
            Meta::Extension { key, value } => Some((key.as_str(), value.as_str())),
            _ => None,
        })
        .collect();
    if !fm.is_empty() {
        out.push_str("---\n");
        for (k, v) in &fm {
            out.push_str(&format!("{}: {}\n", k, v));
        }
        out.push_str("---\n\n");
    }
    out.push_str(&nodes(&g.body));
    out
}

// ---------------------------------------------------------------------------
// Span rendering (used by domain.rs for Encode impl)
// ---------------------------------------------------------------------------

/// Render a list of Spans to inline text (markdown-native).
pub fn spans(ss: &[Span]) -> String {
    ss.iter().map(span).collect()
}

/// Render a single Span to inline text.
pub fn span(s: &Span) -> String {
    use crate::semantic::MathDisplay;
    match s {
        Span::TextSpan { text, marks } => apply_marks(text, marks),
        Span::CodeSpan(text) => format!("`{}`", text),
        Span::MathSpan {
            content,
            display: MathDisplay::InlineMath,
        } => format!("${}$", content),
        Span::MathSpan {
            content,
            display: MathDisplay::DisplayMath,
        } => format!("$${}$$", content),
        Span::LinkSpan { url, children, .. } => {
            if let Some(target) = url.strip_prefix("wiki:") {
                let display = spans(children);
                if display == target {
                    format!("[[{}]]", target)
                } else {
                    format!("[[{}|{}]]", target, display)
                }
            } else {
                format!("[{}]({})", spans(children), url)
            }
        }
        Span::ImageSpan { url, title, alt } => {
            format!("![{}]({} \"{}\")", spans(alt), url, title)
        }
        Span::RefSpan { display, .. } => spans(display),
        Span::EmojiSpan { unicode, .. } => unicode.clone(),
        Span::SpoilerSpan(children) => format!("||{}||", spans(children)),
        Span::HardBreak => "\\\n".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Internal block types — state machine only
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
#[allow(clippy::enum_variant_names)]
enum Block {
    Section {
        level: usize,
        title: Vec<Span>,
        children: Vec<Block>,
        meta: Vec<Meta>,
    },
    Paragraph {
        children: Vec<Span>,
        meta: Vec<Meta>,
    },
    CodeBlock {
        language: String,
        content: String,
        meta: Vec<Meta>,
    },
    Quote {
        children: Vec<Block>,
        attribution: Option<Vec<Span>>,
        meta: Vec<Meta>,
    },
    Callout {
        kind: CalloutKind,
        title: String,
        children: Vec<Block>,
        meta: Vec<Meta>,
    },
    List {
        style: ListStyle,
        start: usize,
        items: Vec<ListItem>,
        meta: Vec<Meta>,
    },
    Separator {
        meta: Vec<Meta>,
    },
    Breath {
        meta: Vec<Meta>,
    },
}

impl Block {
    fn para(children: Vec<Span>) -> Self {
        Block::Paragraph { children, meta: vec![] }
    }
}

#[derive(Clone, Debug)]
struct ListItem {
    children: Vec<Block>,
    checked: Option<bool>,
    meta: Vec<Meta>,
}

fn metadata_to_meta(metadata: Vec<(String, String)>) -> Vec<Meta> {
    metadata
        .into_iter()
        .map(|(key, value)| Meta::Extension { key, value })
        .collect()
}

// ---------------------------------------------------------------------------
// Pass 1: events → flat Vec<Block>
// ---------------------------------------------------------------------------

struct InlineAcc {
    spans: Vec<Span>,
    marks: MarkSet,
}

impl InlineAcc {
    fn new() -> Self {
        InlineAcc { spans: Vec::new(), marks: MarkSet::new() }
    }

    fn push_text(&mut self, text: &str) {
        if self.marks.is_empty() {
            self.spans.push(Span::plain(text));
        } else {
            self.spans.push(Span::TextSpan {
                text: text.to_string(),
                marks: self.marks.clone(),
            });
        }
    }

    fn push_span(&mut self, span: Span) {
        self.spans.push(span);
    }
}

enum BlockCtx {
    Paragraph(InlineAcc),
    Heading(usize, InlineAcc),
    CodeBlock { language: String, content: String },
    Blockquote(Vec<Block>),
    List { style: ListStyle, start: usize, items: Vec<ListItem> },
    ListItem(Vec<Block>),
    Link { url: String, acc: InlineAcc },
}

fn collect_blocks(parser: Parser) -> Vec<Block> {
    let mut output: Vec<Block> = Vec::new();
    let mut stack: Vec<BlockCtx> = Vec::new();
    let mut pending_meta: Vec<Meta> = Vec::new();

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Paragraph => {
                    stack.push(BlockCtx::Paragraph(InlineAcc::new()));
                }
                Tag::Heading { level, .. } => {
                    let lvl = level as usize;
                    stack.push(BlockCtx::Heading(lvl, InlineAcc::new()));
                }
                Tag::CodeBlock(kind) => {
                    let language = match kind {
                        CodeBlockKind::Fenced(lang) => lang.to_string(),
                        CodeBlockKind::Indented => String::new(),
                    };
                    stack.push(BlockCtx::CodeBlock { language, content: String::new() });
                }
                Tag::BlockQuote(_) => {
                    stack.push(BlockCtx::Blockquote(Vec::new()));
                }
                Tag::List(first_item) => {
                    let (style, start) = match first_item {
                        Some(n) => (ListStyle::Ordered, n as usize),
                        None => (ListStyle::Unordered, 1),
                    };
                    stack.push(BlockCtx::List { style, start, items: Vec::new() });
                }
                Tag::Item => {
                    stack.push(BlockCtx::ListItem(Vec::new()));
                }
                Tag::Emphasis => {
                    if let Some(acc) = current_inline_acc(&mut stack) {
                        acc.marks.insert(Mark::Emphasis);
                    }
                }
                Tag::Strong => {
                    if let Some(acc) = current_inline_acc(&mut stack) {
                        acc.marks.insert(Mark::Strong);
                    }
                }
                Tag::Strikethrough => {
                    if let Some(acc) = current_inline_acc(&mut stack) {
                        acc.marks.insert(Mark::Strikethrough);
                    }
                }
                Tag::Link { dest_url, .. } => {
                    ensure_inline_ctx(&mut stack);
                    stack.push(BlockCtx::Link {
                        url: dest_url.to_string(),
                        acc: InlineAcc::new(),
                    });
                }
                _ => {}
            },

            Event::End(tag_end) => match tag_end {
                TagEnd::Paragraph => {
                    if let Some(BlockCtx::Paragraph(acc)) = stack.pop() {
                        let meta = std::mem::take(&mut pending_meta);
                        let block = Block::Paragraph { children: acc.spans, meta };
                        push_block(&mut output, &mut stack, block);
                    }
                }
                TagEnd::Heading(_) => {
                    if let Some(BlockCtx::Heading(level, acc)) = stack.pop() {
                        let meta = std::mem::take(&mut pending_meta);
                        let block = Block::Section { level, title: acc.spans, children: Vec::new(), meta };
                        push_block(&mut output, &mut stack, block);
                    }
                }
                TagEnd::CodeBlock => {
                    if let Some(BlockCtx::CodeBlock { language, content }) = stack.pop() {
                        let trimmed = content.strip_suffix('\n').unwrap_or(&content).to_string();
                        let meta = std::mem::take(&mut pending_meta);
                        let block = Block::CodeBlock { language, content: trimmed, meta };
                        push_block(&mut output, &mut stack, block);
                    }
                }
                TagEnd::BlockQuote(_) => {
                    if let Some(BlockCtx::Blockquote(children)) = stack.pop() {
                        let meta = std::mem::take(&mut pending_meta);
                        let block = Block::Quote { children, attribution: None, meta };
                        push_block(&mut output, &mut stack, block);
                    }
                }
                TagEnd::List(_) => {
                    if let Some(BlockCtx::List { style, start, items }) = stack.pop() {
                        let meta = std::mem::take(&mut pending_meta);
                        let block = Block::List { style, start, items, meta };
                        push_block(&mut output, &mut stack, block);
                    }
                }
                TagEnd::Item => {
                    if matches!(stack.last(), Some(BlockCtx::Paragraph(_))) {
                        if let Some(BlockCtx::Paragraph(acc)) = stack.pop() {
                            if !acc.spans.is_empty() {
                                if let Some(BlockCtx::ListItem(children)) = stack.last_mut() {
                                    children.push(Block::para(acc.spans));
                                }
                            }
                        }
                    }
                    if let Some(BlockCtx::ListItem(children)) = stack.pop() {
                        let item = ListItem { children, checked: None, meta: vec![] };
                        if let Some(BlockCtx::List { items, .. }) = stack.last_mut() {
                            items.push(item);
                        }
                    }
                }
                TagEnd::Emphasis => {
                    if let Some(acc) = current_inline_acc(&mut stack) {
                        acc.marks.remove(&Mark::Emphasis);
                    }
                }
                TagEnd::Strong => {
                    if let Some(acc) = current_inline_acc(&mut stack) {
                        acc.marks.remove(&Mark::Strong);
                    }
                }
                TagEnd::Strikethrough => {
                    if let Some(acc) = current_inline_acc(&mut stack) {
                        acc.marks.remove(&Mark::Strikethrough);
                    }
                }
                TagEnd::Link => {
                    if let Some(BlockCtx::Link { url, acc }) = stack.pop() {
                        let link_span = Span::LinkSpan {
                            url,
                            title: String::new(),
                            children: acc.spans,
                        };
                        if let Some(parent_acc) = current_inline_acc(&mut stack) {
                            parent_acc.push_span(link_span);
                        }
                    }
                }
                _ => {}
            },

            Event::Text(text) => {
                if let Some(BlockCtx::CodeBlock { content, .. }) = stack.last_mut() {
                    content.push_str(&text);
                } else {
                    ensure_inline_ctx(&mut stack);
                    if let Some(acc) = current_inline_acc(&mut stack) {
                        acc.push_text(&text);
                    }
                }
            }

            Event::Code(code) => {
                ensure_inline_ctx(&mut stack);
                if let Some(acc) = current_inline_acc(&mut stack) {
                    acc.push_span(Span::CodeSpan(code.to_string()));
                }
            }

            Event::HardBreak => {
                if let Some(acc) = current_inline_acc(&mut stack) {
                    acc.push_span(Span::HardBreak);
                }
            }

            Event::SoftBreak => {
                if let Some(acc) = current_inline_acc(&mut stack) {
                    acc.push_span(Span::plain("\n"));
                }
            }

            Event::Rule => {
                let meta = std::mem::take(&mut pending_meta);
                let block = Block::Separator { meta };
                push_block(&mut output, &mut stack, block);
            }

            Event::Html(text) => {
                pending_meta.extend(parse_html_comment(&text));
            }

            _ => {}
        }
    }

    output
}

// ---------------------------------------------------------------------------
// HTML comment → Meta parsing
// ---------------------------------------------------------------------------

fn parse_html_comment(html: &str) -> Vec<Meta> {
    let trimmed = html.trim();
    let inner = match trimmed.strip_prefix("<!--") {
        Some(s) => match s.strip_suffix("-->") {
            Some(s) => s.trim(),
            None => return vec![],
        },
        None => return vec![],
    };

    if let Some((id_part, role_part)) = inner.split_once(", role: ") {
        if let Some(id) = id_part.strip_prefix("id: ") {
            if let Some(role) = parse_role(role_part) {
                return vec![Meta::Id(id.to_string()), Meta::Role(role)];
            }
        }
    }

    if let Some(id) = inner.strip_prefix("id: ") {
        return vec![Meta::Id(id.to_string())];
    }
    if let Some(role_str) = inner.strip_prefix("role: ") {
        if let Some(role) = parse_role(role_str) {
            return vec![Meta::Role(role)];
        }
    }

    if let Some((key, value)) = inner.split_once(": ") {
        return vec![Meta::Extension {
            key: key.to_string(),
            value: value.to_string(),
        }];
    }

    vec![]
}

fn parse_role(s: &str) -> Option<Role> {
    match s {
        "claim" => Some(Role::Claim),
        "evidence" => Some(Role::Evidence),
        "example" => Some(Role::Example),
        "aside" => Some(Role::Aside),
        "defining" => Some(Role::Defining),
        "instruction" => Some(Role::Instruction),
        "summary" => Some(Role::Summary),
        "transition" => Some(Role::Transition),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Frontmatter + wikilinks
// ---------------------------------------------------------------------------

fn extract_frontmatter(input: &str) -> (Vec<(String, String)>, &str) {
    if !input.starts_with("---\n") {
        return (vec![], input);
    }
    let after_first = &input[4..];
    let (yaml_end, rest_start) = if let Some(pos) = after_first.find("\n---\n") {
        (pos, pos + 5)
    } else if after_first.ends_with("\n---") {
        (after_first.len() - 4, after_first.len())
    } else {
        return (vec![], input);
    };

    let yaml = &after_first[..yaml_end];
    let rest = &after_first[rest_start..];
    let rest = rest.strip_prefix('\n').unwrap_or(rest);

    let metadata: Vec<(String, String)> = yaml
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(": ")?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect();

    (metadata, rest)
}

fn preprocess_wikilinks(input: &str) -> String {
    let mut result = String::new();
    let mut rest = input;
    while let Some(start) = rest.find("[[") {
        result.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        if let Some(end) = after.find("]]") {
            let content = &after[..end];
            if let Some(pipe) = content.find('|') {
                let target = &content[..pipe];
                let display = &content[pipe + 1..];
                result.push_str(&format!("[{}](wiki:{})", display, target));
            } else {
                result.push_str(&format!("[{}](wiki:{})", content, content));
            }
            rest = &after[end + 2..];
        } else {
            result.push_str("[[");
            rest = after;
        }
    }
    result.push_str(rest);
    result
}

// ---------------------------------------------------------------------------
// Gestalt-specific post-processing
// ---------------------------------------------------------------------------

fn postprocess_gestalt(blocks: Vec<Block>) -> Vec<Block> {
    blocks
        .into_iter()
        .map(|b| match &b {
            Block::Paragraph { children, .. } if is_breath(children) => {
                if let Block::Paragraph { meta, .. } = b {
                    Block::Breath { meta }
                } else {
                    unreachable!()
                }
            }
            Block::Quote { children, .. } if is_callout_quote(children) => {
                if let Block::Quote { children, meta, .. } = b {
                    convert_callout(children, meta)
                } else {
                    unreachable!()
                }
            }
            _ => b,
        })
        .collect()
}

fn is_breath(children: &[Span]) -> bool {
    children.len() == 1
        && matches!(
            &children[0],
            Span::TextSpan { text, marks } if text == ".." && marks.is_empty()
        )
}

fn is_callout_quote(children: &[Block]) -> bool {
    if let Some(Block::Paragraph { children: spans, .. }) = children.first() {
        return extract_callout_header(spans).is_some();
    }
    false
}

fn extract_callout_header(spans: &[Span]) -> Option<(CalloutKind, String, usize)> {
    let mut header_text = String::new();
    let mut span_count = 0;
    for span in spans {
        match span {
            Span::TextSpan { text, .. } => {
                if text == "\n" {
                    break;
                }
                header_text.push_str(text);
                span_count += 1;
            }
            _ => break,
        }
    }

    if !header_text.starts_with("[!") {
        return None;
    }
    let bracket_end = header_text.find(']')?;
    let kind_str = &header_text[2..bracket_end];
    let title = header_text[bracket_end + 1..].trim().to_string();

    let kind = match kind_str.to_uppercase().as_str() {
        "TIP" => CalloutKind::Tip,
        "IMPORTANT" => CalloutKind::Important,
        "WARNING" => CalloutKind::Warning,
        "CAUTION" => CalloutKind::Caution,
        _ => CalloutKind::Note,
    };

    Some((kind, title, span_count))
}

fn convert_callout(mut children: Vec<Block>, meta: Vec<Meta>) -> Block {
    let first_para = children.remove(0);
    let Block::Paragraph { children: spans, .. } = first_para else {
        unreachable!()
    };

    let (kind, title, span_count) = extract_callout_header(&spans).unwrap();

    let remaining_spans: Vec<Span> = spans[span_count..]
        .iter()
        .skip_while(|s| matches!(s, Span::TextSpan { text, .. } if text == "\n"))
        .cloned()
        .collect();

    if !remaining_spans.is_empty() {
        children.insert(0, Block::para(remaining_spans));
    }

    Block::Callout { kind, title, children, meta }
}

// ---------------------------------------------------------------------------
// Stack helpers
// ---------------------------------------------------------------------------

fn ensure_inline_ctx(stack: &mut Vec<BlockCtx>) {
    if current_inline_acc(stack).is_some() {
        return;
    }
    if matches!(stack.last(), Some(BlockCtx::ListItem(_))) {
        stack.push(BlockCtx::Paragraph(InlineAcc::new()));
    }
}

fn current_inline_acc(stack: &mut [BlockCtx]) -> Option<&mut InlineAcc> {
    for ctx in stack.iter_mut().rev() {
        match ctx {
            BlockCtx::Paragraph(acc) | BlockCtx::Heading(_, acc) | BlockCtx::Link { acc, .. } => {
                return Some(acc);
            }
            _ => {}
        }
    }
    None
}

fn push_block(output: &mut Vec<Block>, stack: &mut [BlockCtx], block: Block) {
    for ctx in stack.iter_mut().rev() {
        match ctx {
            BlockCtx::Blockquote(children) | BlockCtx::ListItem(children) => {
                children.push(block);
                return;
            }
            _ => {}
        }
    }
    output.push(block);
}

// ---------------------------------------------------------------------------
// Pass 2: nest sections
// ---------------------------------------------------------------------------

fn nest_sections(blocks: Vec<Block>) -> Vec<Block> {
    let mut result: Vec<Block> = Vec::new();
    let mut i = 0;
    while i < blocks.len() {
        match &blocks[i] {
            Block::Section { level, .. } => {
                let section_level = *level;
                let heading = blocks[i].clone();
                i += 1;
                let mut children = Vec::new();
                while i < blocks.len() {
                    match &blocks[i] {
                        Block::Section { level: next_lvl, .. } if *next_lvl <= section_level => {
                            break;
                        }
                        _ => {
                            children.push(blocks[i].clone());
                            i += 1;
                        }
                    }
                }
                let nested_children = nest_sections(children);
                if let Block::Section { level, title, meta, .. } = heading {
                    result.push(Block::Section { level, title, children: nested_children, meta });
                }
            }
            _ => {
                result.push(blocks[i].clone());
                i += 1;
            }
        }
    }
    result
}

// ---------------------------------------------------------------------------
// Lift: Block → domain::Node<Document>
// ---------------------------------------------------------------------------

fn lift_gestalt_internal(blocks: Vec<Block>, head: Vec<Meta>) -> domain::Gestalt<domain::Document> {
    domain::Gestalt {
        domain: domain::Document,
        head,
        body: blocks.iter().map(lift_block).collect(),
    }
}

fn lift_block(block: &Block) -> domain::Node<domain::Document> {
    match block {
        Block::Section { level, title, children, meta } => domain::Node {
            meta: meta.clone(),
            children: children.iter().map(lift_block).collect(),
            kind: domain::DocumentKind::Section { level: *level, title: title.clone() },
        },

        Block::Paragraph { children, meta } => domain::Node {
            meta: meta.clone(),
            children: vec![],
            kind: domain::DocumentKind::Paragraph { content: children.clone() },
        },

        Block::CodeBlock { language, content, meta } => domain::Node {
            meta: meta.clone(),
            children: vec![],
            kind: domain::DocumentKind::CodeBlock {
                language: language.clone(),
                content: content.clone(),
            },
        },

        Block::Quote { children, attribution, meta } => domain::Node {
            meta: meta.clone(),
            children: children.iter().map(lift_block).collect(),
            kind: domain::DocumentKind::Quote { attribution: attribution.clone() },
        },

        Block::Callout { kind, title, children, meta } => domain::Node {
            meta: meta.clone(),
            children: children.iter().map(lift_block).collect(),
            kind: domain::DocumentKind::Callout { kind: kind.clone(), title: title.clone() },
        },

        Block::List { style, start, items, meta } => domain::Node {
            meta: meta.clone(),
            children: items.iter().map(lift_list_item).collect(),
            kind: domain::DocumentKind::List { style: style.clone(), start: *start },
        },

        Block::Separator { meta } => domain::Node {
            meta: meta.clone(),
            children: vec![],
            kind: domain::DocumentKind::Separator,
        },

        Block::Breath { meta } => domain::Node {
            meta: meta.clone(),
            children: vec![],
            kind: domain::DocumentKind::Breath,
        },
    }
}

fn lift_list_item(item: &ListItem) -> domain::Node<domain::Document> {
    domain::Node {
        meta: item.meta.clone(),
        children: item.children.iter().map(lift_block).collect(),
        kind: domain::DocumentKind::ListItem { checked: item.checked },
    }
}

// ---------------------------------------------------------------------------
// Markdown renderer (to_markdown helpers)
// ---------------------------------------------------------------------------

fn nodes(ns: &[domain::Node<domain::Document>]) -> String {
    ns.iter().map(node).collect::<Vec<_>>().join("\n")
}

fn node(n: &domain::Node<domain::Document>) -> String {
    let comment = render_meta(&n.meta);
    match &n.kind {
        domain::DocumentKind::Section { level, title } => {
            let hashes = "#".repeat(*level);
            let mut out = format!("{}{} {}\n", comment, hashes, spans(title));
            if !n.children.is_empty() {
                out.push('\n');
                out.push_str(&nodes(&n.children));
            }
            out
        }

        domain::DocumentKind::Paragraph { content } => {
            format!("{}{}\n", comment, spans(content))
        }

        domain::DocumentKind::CodeBlock { language, content } => {
            format!("{}```{}\n{}\n```\n", comment, language, content)
        }

        domain::DocumentKind::Quote { attribution } => {
            let inner = nodes(&n.children);
            let mut out = comment;
            out.push_str(&prefix_lines(&inner, "> "));
            if let Some(attr_spans) = attribution {
                out.push_str(">\n");
                out.push_str(&format!("> \u{2014} {}\n", spans(attr_spans)));
            }
            out
        }

        domain::DocumentKind::Callout { kind, title } => {
            let kind_upper = callout_kind_str(kind).to_uppercase();
            let mut header = format!("> [!{}]", kind_upper);
            if !title.is_empty() {
                header.push_str(&format!(" {}", title));
            }
            header.push('\n');
            if !n.children.is_empty() {
                let inner = nodes(&n.children);
                header.push_str(&prefix_lines(&inner, "> "));
            }
            format!("{}{}", comment, header)
        }

        domain::DocumentKind::List { style, start } => {
            let mut out = comment;
            for (i, child) in n.children.iter().enumerate() {
                let number = *start + i;
                out.push_str(&list_item(child, style, number));
            }
            out
        }

        domain::DocumentKind::ListItem { .. } => nodes(&n.children),

        domain::DocumentKind::DefinitionList => {
            let mut out = comment;
            for child in &n.children {
                out.push_str(&node(child));
            }
            out
        }

        domain::DocumentKind::Table { columns } => {
            let mut out = comment;
            if let Some(head_row) = n.children.first() {
                let cell_str = |cell: &domain::Node<domain::Document>| {
                    if let domain::DocumentKind::Paragraph { content } = &cell.kind {
                        format!(" {} |", spans(content))
                    } else {
                        " |".to_string()
                    }
                };
                out.push_str(&format!(
                    "|{}\n",
                    head_row.children.iter().map(cell_str).collect::<String>()
                ));
                out.push_str(&format!(
                    "|{}\n",
                    columns
                        .iter()
                        .map(|align| match align {
                            ColumnAlign::Left => ":---|",
                            ColumnAlign::Center => ":---:|",
                            ColumnAlign::Right => "---:|",
                            ColumnAlign::Default => "---|",
                        })
                        .collect::<String>()
                ));
                for row in &n.children[1..] {
                    out.push_str(&format!(
                        "|{}\n",
                        row.children.iter().map(cell_str).collect::<String>()
                    ));
                }
            }
            out
        }

        domain::DocumentKind::Figure { caption } => {
            let mut out = comment;
            if let Some(content_node) = n.children.first() {
                out.push_str(&node(content_node));
            }
            if let Some(cap_spans) = caption {
                out.push_str(&format!("*{}*\n", spans(cap_spans)));
            }
            out
        }

        domain::DocumentKind::Separator => format!("{}---\n", comment),

        domain::DocumentKind::Breath => format!("{}..\n", comment),

        domain::DocumentKind::RawBlock { content, format: fmt } => {
            format!("{}```{}\n{}\n```\n", comment, fmt, content)
        }

        domain::DocumentKind::Embedded(gestalt) => to_markdown(gestalt),
    }
}

fn list_item(item: &domain::Node<domain::Document>, style: &ListStyle, number: usize) -> String {
    let marker = match style {
        ListStyle::Unordered => "- ".to_string(),
        ListStyle::Ordered => format!("{}. ", number),
    };
    let checked = match &item.kind {
        domain::DocumentKind::ListItem { checked: Some(true) } => "[x] ",
        domain::DocumentKind::ListItem { checked: Some(false) } => "[ ] ",
        _ => "",
    };

    if item.children.is_empty() {
        return format!("{}{}\n", marker, checked);
    }

    let first_text = match &item.children[0].kind {
        domain::DocumentKind::Paragraph { content } if item.children[0].meta.is_empty() => {
            spans(content)
        }
        _ => node(&item.children[0]).trim_end().to_string(),
    };
    let mut out = format!("{}{}{}\n", marker, checked, first_text);

    let indent = " ".repeat(marker.len());
    for child in &item.children[1..] {
        out.push('\n');
        let child_text = node(child);
        for line in child_text.lines() {
            if line.is_empty() {
                out.push('\n');
            } else {
                out.push_str(&indent);
                out.push_str(line);
                out.push('\n');
            }
        }
    }

    out
}

fn prefix_lines(text: &str, prefix: &str) -> String {
    let mut out: Vec<String> = text
        .lines()
        .map(|l| {
            if l.is_empty() {
                ">".to_string()
            } else {
                format!("{}{}", prefix, l)
            }
        })
        .collect();
    if !out.is_empty() {
        let last = out.len() - 1;
        out[last].push('\n');
        out.join("\n")
    } else {
        String::new()
    }
}

fn apply_marks(text: &str, marks: &MarkSet) -> String {
    let mut t = text.to_string();
    if marks.contains(&Mark::Subscript) {
        t = format!("~{}~", t);
    }
    if marks.contains(&Mark::Superscript) {
        t = format!("^{}^", t);
    }
    if marks.contains(&Mark::Emphasis) {
        t = format!("*{}*", t);
    }
    if marks.contains(&Mark::Strong) {
        t = format!("**{}**", t);
    }
    if marks.contains(&Mark::Highlight) {
        t = format!("=={}==", t);
    }
    if marks.contains(&Mark::Strikethrough) {
        t = format!("~~{}~~", t);
    }
    t
}

pub(crate) fn render_meta(meta: &[Meta]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < meta.len() {
        match (&meta[i], meta.get(i + 1)) {
            (Meta::Id(id), Some(Meta::Role(role))) => {
                out.push_str(&format!("<!-- id: {}, role: {} -->\n", id, role_str(role)));
                i += 2;
            }
            (Meta::Id(id), _) => {
                out.push_str(&format!("<!-- id: {} -->\n", id));
                i += 1;
            }
            (Meta::Role(role), _) => {
                out.push_str(&format!("<!-- role: {} -->\n", role_str(role)));
                i += 1;
            }
            (Meta::Extension { key, value }, _) => {
                out.push_str(&format!("<!-- {}: {} -->\n", key, value));
                i += 1;
            }
        }
    }
    out
}

fn role_str(role: &Role) -> &'static str {
    match role {
        Role::Claim => "claim",
        Role::Evidence => "evidence",
        Role::Example => "example",
        Role::Aside => "aside",
        Role::Defining => "defining",
        Role::Instruction => "instruction",
        Role::Summary => "summary",
        Role::Transition => "transition",
    }
}

fn callout_kind_str(kind: &CalloutKind) -> &'static str {
    match kind {
        CalloutKind::Note => "note",
        CalloutKind::Tip => "tip",
        CalloutKind::Important => "important",
        CalloutKind::Warning => "warning",
        CalloutKind::Caution => "caution",
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::DocumentKind;
    use crate::dom::DOM as _;

    #[test]
    fn parse_empty_markdown() {
        let g = from_markdown("");
        assert!(g.body.is_empty());
    }

    #[test]
    fn parse_simple_paragraph() {
        let g = from_markdown("Hello world\n");
        assert_eq!(g.body.len(), 1);
        assert!(matches!(&g.body[0].kind, DocumentKind::Paragraph { .. }));
    }

    #[test]
    fn parse_heading() {
        let g = from_markdown("# Title\n\nParagraph\n");
        // Section wraps paragraph
        assert_eq!(g.body.len(), 1);
        assert!(matches!(&g.body[0].kind, DocumentKind::Section { level: 1, .. }));
        assert_eq!(g.body[0].children.len(), 1);
    }

    #[test]
    fn parse_code_block() {
        let g = from_markdown("```rust\nlet x = 1;\n```\n");
        assert_eq!(g.body.len(), 1);
        assert!(matches!(&g.body[0].kind, DocumentKind::CodeBlock { language, .. } if language == "rust"));
    }

    #[test]
    fn parse_separator() {
        let g = from_markdown("---\n");
        assert_eq!(g.body.len(), 1);
        assert!(matches!(&g.body[0].kind, DocumentKind::Separator));
    }

    #[test]
    fn parse_unordered_list() {
        let g = from_markdown("- item one\n- item two\n");
        assert_eq!(g.body.len(), 1);
        assert!(matches!(&g.body[0].kind, DocumentKind::List { style: ListStyle::Unordered, .. }));
        assert_eq!(g.body[0].children.len(), 2);
    }

    #[test]
    fn parse_ordered_list() {
        let g = from_markdown("1. first\n2. second\n");
        assert_eq!(g.body.len(), 1);
        assert!(matches!(&g.body[0].kind, DocumentKind::List { style: ListStyle::Ordered, .. }));
    }

    #[test]
    fn parse_yaml_frontmatter() {
        let g = from_markdown("---\ntitle: Hello\nauthor: Reed\n---\n\nBody text\n");
        assert!(!g.head.is_empty());
        assert!(g.head.iter().any(|m| matches!(m, Meta::Extension { key, .. } if key == "title")));
    }

    #[test]
    fn parse_wiki_link() {
        let g = from_markdown("See [[Target]]\n");
        assert_eq!(g.body.len(), 1);
        if let DocumentKind::Paragraph { content } = &g.body[0].kind {
            let has_link = content.iter().any(|s| matches!(s, Span::LinkSpan { url, .. } if url.starts_with("wiki:")));
            assert!(has_link);
        } else {
            panic!("expected paragraph");
        }
    }

    #[test]
    fn parse_gestalt_callout() {
        let input = "> [!NOTE] Title\n> Body text\n";
        let g = from_gestalt(input);
        assert_eq!(g.body.len(), 1);
        assert!(matches!(&g.body[0].kind, DocumentKind::Callout { kind: CalloutKind::Note, .. }));
    }

    #[test]
    fn parse_gestalt_breath() {
        let input = "..\n";
        let g = from_gestalt(input);
        assert_eq!(g.body.len(), 1);
        assert!(matches!(&g.body[0].kind, DocumentKind::Breath));
    }

    #[test]
    fn roundtrip_paragraph() {
        let input = "Hello world\n";
        let g = from_markdown(input);
        let out = to_markdown(&g);
        assert!(out.contains("Hello world"));
    }

    #[test]
    fn roundtrip_heading() {
        let g = from_markdown("# Section Title\n");
        let out = to_markdown(&g);
        assert!(out.contains("# Section Title"));
    }

    #[test]
    fn roundtrip_code_block() {
        let g = from_markdown("```rust\nlet x = 1;\n```\n");
        let out = to_markdown(&g);
        assert!(out.contains("```rust"));
        assert!(out.contains("let x = 1;"));
    }

    #[test]
    fn roundtrip_separator() {
        let g = from_markdown("---\n");
        let out = to_markdown(&g);
        assert!(out.contains("---"));
    }

    #[test]
    fn gestalt_is_content_addressed() {
        let a = from_markdown("Hello\n");
        let b = from_markdown("Hello\n");
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn different_content_different_oid() {
        let a = from_markdown("Hello\n");
        let b = from_markdown("World\n");
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn span_plain_renders() {
        let s = Span::plain("hello");
        assert_eq!(span(&s), "hello");
    }

    #[test]
    fn span_strong_renders() {
        let s = Span::marked("hello", Mark::Strong);
        assert_eq!(span(&s), "**hello**");
    }

    #[test]
    fn span_emphasis_renders() {
        let s = Span::marked("hello", Mark::Emphasis);
        assert_eq!(span(&s), "*hello*");
    }

    #[test]
    fn span_code_renders() {
        let s = Span::CodeSpan("x + 1".into());
        assert_eq!(span(&s), "`x + 1`");
    }
}
