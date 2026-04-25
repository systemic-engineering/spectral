//! License boundary tests: apache2/ must never import from sel/.

use std::fs;
use std::path::Path;

#[test]
fn apache2_never_imports_sel() {
    let apache2_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/apache2");
    assert!(apache2_dir.exists(), "src/apache2/ not found");

    let mut violations = Vec::new();
    for entry in fs::read_dir(&apache2_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") { continue; }
        let content = fs::read_to_string(&path).unwrap();
        let filename = path.file_name().unwrap().to_string_lossy().to_string();
        for (line_num, line) in content.lines().enumerate() {
            if line.contains("crate::sel") || line.contains("super::sel") {
                violations.push(format!("{}:{}: {}", filename, line_num + 1, line.trim()));
            }
        }
    }
    assert!(violations.is_empty(),
        "LICENSE BOUNDARY VIOLATION: apache2/ imports from sel/:\n{}", violations.join("\n"));
}

#[test]
fn apache2_compiles_without_sel_feature() {
    // If apache2/ depends on sel/, this won't compile without --features sel
    use spectral::apache2::runtime::Runtime;
    use spectral::apache2::identity::Name;
    use spectral::apache2::loss::InitLoss;
    use spectral::apache2::signal::Signal;
    use spectral::apache2::observe::Observation;
    // All types accessible without sel feature
    let _ = (
        std::any::type_name::<fn() -> InitLoss>(),
        std::any::type_name::<fn() -> Signal>(),
        std::any::type_name::<fn() -> Observation>(),
    );
}
