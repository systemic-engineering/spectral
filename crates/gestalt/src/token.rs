//! Token — a design token is a named lambda over TokenValue.
//!
//! `Token = Named<Lambda<TokenValue>>`. Materialization is beta reduction.
//! Theme is the observation frame. Same token + same theme = same Oid.

use prism_core::lambda::{reduce_bounded, Lambda, ReductionError, ReductionLoss};
use prism_core::named::Named;
use prism_core::oid::{Addressable, Oid};
use terni::Imperfect;

// ---------------------------------------------------------------------------
// Token value types
// ---------------------------------------------------------------------------

/// What a design token reduces to.
#[derive(Clone, Debug, PartialEq)]
pub enum TokenValue {
    Color(Hsl),
    Dimension(Dimension),
    Duration(u64),
    Weight(i32),
    Family(String),
    Radius(String),
    Shadow(String),
    Easing(String),
}

/// HSL color.
#[derive(Clone, Debug, PartialEq)]
pub struct Hsl {
    pub h: f64,
    pub s: f64,
    pub l: f64,
}

/// A dimension with unit.
#[derive(Clone, Debug, PartialEq)]
pub struct Dimension {
    pub value: f64,
    pub unit: DimensionUnit,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DimensionUnit {
    Px,
    Rem,
    Em,
    Percent,
}

// ---------------------------------------------------------------------------
// Theme — the observation frame
// ---------------------------------------------------------------------------

/// Theme: the observation frame for token materialization.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    pub mode: Mode,
    pub density: Density,
    pub contrast: f64,
    pub scale: f64,
    pub motion: Motion,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Density {
    Compact,
    Comfortable,
    Spacious,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Motion {
    Full,
    Reduced,
}

impl Theme {
    /// Default theme: light, comfortable, standard contrast/scale, full motion.
    pub fn default_theme() -> Self {
        Theme {
            mode: Mode::Light,
            density: Density::Comfortable,
            contrast: 4.5,
            scale: 1.0,
            motion: Motion::Full,
        }
    }
}

// ---------------------------------------------------------------------------
// Token = Named<Lambda<TokenValue>>
// ---------------------------------------------------------------------------

/// A design token. Named lambda over TokenValue.
/// The name is for humans. The Oid is for machines.
pub type Token = Named<Lambda<TokenValue>>;

/// Create a token from a name and lambda.
pub fn token(name: &'static str, lambda: Lambda<TokenValue>) -> Token {
    Named(name, lambda)
}

/// Encode a theme axis as a token value for pattern matching.
pub fn mode_value(mode: &Mode) -> TokenValue {
    match mode {
        Mode::Light => TokenValue::Family("light".into()),
        Mode::Dark => TokenValue::Family("dark".into()),
    }
}

/// Create a mode-switching token: two values, one for light, one for dark.
pub fn by_mode(name: &'static str, light: TokenValue, dark: TokenValue) -> Token {
    let theme_param = Oid::hash(b"@theme.mode");
    let lambda = Lambda::abs(
        theme_param.clone(),
        Lambda::case(
            Lambda::bind(theme_param),
            vec![
                (
                    prism_core::lambda::Pattern::Exact(mode_value(&Mode::Light)),
                    Lambda::bind(Oid::hash(
                        format!("{}:light", name).as_bytes(),
                    )),
                ),
                (
                    prism_core::lambda::Pattern::Exact(mode_value(&Mode::Dark)),
                    Lambda::bind(Oid::hash(
                        format!("{}:dark", name).as_bytes(),
                    )),
                ),
            ],
        ),
    );
    Named(name, lambda)
}

/// Materialize a token: reduce(apply(token_lambda, theme_value), budget).
///
/// This is beta reduction with a budget. The token's lambda is applied to
/// a theme value, and the result is reduced to normal form.
pub fn materialize(
    token: &Token,
    _theme: &Theme,
    budget: usize,
) -> Imperfect<Lambda<TokenValue>, ReductionError, ReductionLoss> {
    // For now, reduce the token's lambda directly.
    // Full materialization would encode the theme as a lambda value and apply.
    reduce_bounded(token.inner().clone(), budget)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use prism_core::oid::Addressable;

    #[test]
    fn token_creation() {
        let t = token(
            "background",
            Lambda::bind(Oid::hash(b"@color/background")),
        );
        assert_eq!(t.name(), "background");
        assert!(!t.oid().is_dark());
    }

    #[test]
    fn token_content_addressed() {
        let a = token("bg", Lambda::bind(Oid::hash(b"@color/bg")));
        let b = token("bg", Lambda::bind(Oid::hash(b"@color/bg")));
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn different_token_different_oid() {
        let a = token("bg", Lambda::bind(Oid::hash(b"@color/bg")));
        let b = token("fg", Lambda::bind(Oid::hash(b"@color/fg")));
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn same_lambda_different_name_different_oid() {
        let lambda = Lambda::bind(Oid::hash(b"@color/bg"));
        let a = token("background", lambda.clone());
        let b = token("surface", lambda);
        assert_ne!(a.oid(), b.oid());
    }

    #[test]
    fn theme_default() {
        let t = Theme::default_theme();
        assert_eq!(t.mode, Mode::Light);
        assert_eq!(t.density, Density::Comfortable);
        assert_eq!(t.contrast, 4.5);
        assert_eq!(t.scale, 1.0);
        assert_eq!(t.motion, Motion::Full);
    }

    #[test]
    fn materialize_bind_is_normal_form() {
        let t = token("bg", Lambda::bind(Oid::hash(b"@color/bg")));
        let theme = Theme::default_theme();
        let result = materialize(&t, &theme, 100);
        assert!(result.is_ok());
    }

    #[test]
    fn materialize_identity_reduces() {
        let x = Oid::hash(b"x");
        let id_lambda = Lambda::abs(x.clone(), Lambda::bind(x));
        let arg = Lambda::bind(Oid::hash(b"value"));
        let applied = Lambda::apply(id_lambda, arg.clone());

        let t = token("test", applied);
        let theme = Theme::default_theme();
        let result = materialize(&t, &theme, 100);
        assert_eq!(result.ok(), Some(arg));
    }

    #[test]
    fn by_mode_creates_case_lambda() {
        let t = by_mode(
            "background",
            TokenValue::Color(Hsl { h: 0.0, s: 0.0, l: 100.0 }),
            TokenValue::Color(Hsl { h: 240.0, s: 10.0, l: 4.0 }),
        );
        assert_eq!(t.name(), "background");
        assert!(matches!(t.inner(), Lambda::Abs(_)));
    }

    #[test]
    fn by_mode_is_content_addressed() {
        let a = by_mode(
            "bg",
            TokenValue::Color(Hsl { h: 0.0, s: 0.0, l: 100.0 }),
            TokenValue::Color(Hsl { h: 0.0, s: 0.0, l: 0.0 }),
        );
        let b = by_mode(
            "bg",
            TokenValue::Color(Hsl { h: 0.0, s: 0.0, l: 100.0 }),
            TokenValue::Color(Hsl { h: 0.0, s: 0.0, l: 0.0 }),
        );
        assert_eq!(a.oid(), b.oid());
    }

    #[test]
    fn mode_value_light_dark_differ() {
        let l = mode_value(&Mode::Light);
        let d = mode_value(&Mode::Dark);
        assert_ne!(l, d);
    }

    #[test]
    fn hsl_equality() {
        let a = Hsl { h: 0.0, s: 50.0, l: 50.0 };
        let b = Hsl { h: 0.0, s: 50.0, l: 50.0 };
        let c = Hsl { h: 120.0, s: 50.0, l: 50.0 };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn dimension_equality() {
        let a = Dimension { value: 16.0, unit: DimensionUnit::Px };
        let b = Dimension { value: 16.0, unit: DimensionUnit::Px };
        let c = Dimension { value: 16.0, unit: DimensionUnit::Rem };
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn token_value_variants() {
        let color = TokenValue::Color(Hsl { h: 0.0, s: 0.0, l: 0.0 });
        let dim = TokenValue::Dimension(Dimension { value: 1.0, unit: DimensionUnit::Rem });
        let dur = TokenValue::Duration(200);
        let weight = TokenValue::Weight(400);
        let family = TokenValue::Family("sans-serif".into());
        let radius = TokenValue::Radius("4px".into());
        let shadow = TokenValue::Shadow("0 2px 4px rgba(0,0,0,0.1)".into());
        let easing = TokenValue::Easing("ease-in-out".into());

        // All variants are distinct
        assert_ne!(color, dim);
        assert_ne!(dim, dur);
        assert_ne!(dur, weight);
        assert_ne!(weight, family);
        assert_ne!(family, radius);
        assert_ne!(radius, shadow);
        assert_ne!(shadow, easing);
    }
}
