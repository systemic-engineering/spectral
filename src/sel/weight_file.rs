//! Weight file — the .shatter weight file format.
//!
//! COULD MOVE TO MIRROR: The file format specification could become
//! part of the `@shatter` boot grammar. The Luminosity type already
//! lives in prism-core.
//! STAYS IN SPECTRAL: Serialization, deserialization, and disk I/O.
//! The weights are runtime state that spectral manages.
//!
//! Contains Surface, Shatter, and Reflection weights plus gestalt eigenvalues.

use crate::sel::reflection::Reflection;
use crate::sel::shatter_model::Shatter;
use crate::sel::surface::Surface;

// ---------------------------------------------------------------------------
// Luminosity — training state indicator
// ---------------------------------------------------------------------------

/// Training state of the weight file.
/// Maps to prism-core's Luminosity concept.
#[derive(Clone, Debug, PartialEq)]
pub enum WeightLuminosity {
    /// Well-trained, low loss, confident predictions.
    Light,
    /// Partially trained, some topics high-loss.
    Dimmed(f64),
    /// No training data, fresh user, all predictions are guesses.
    Dark,
}

// ---------------------------------------------------------------------------
// WeightState — the complete weight file
// ---------------------------------------------------------------------------

/// The complete .shatter weight file: all three models + gestalt.
#[derive(Clone, Debug, PartialEq)]
pub struct WeightState {
    // Surface weights
    pub surface_w1: Vec<f64>,
    pub surface_b1: Vec<f64>,
    pub surface_w2: Vec<f64>,
    pub surface_b2: Vec<f64>,

    // Shatter weights
    pub shatter_w1: Vec<f64>,
    pub shatter_b1: Vec<f64>,
    pub shatter_w2: Vec<f64>,
    pub shatter_b2: Vec<f64>,
    pub shatter_concept_embed: Vec<f64>,
    pub shatter_slot_embed: Vec<f64>,

    // Reflection weights
    pub reflection_w1: Vec<f64>,
    pub reflection_b1: Vec<f64>,
    pub reflection_w2: Vec<f64>,
    pub reflection_b2: Vec<f64>,

    // Gestalt
    pub eigenvalues: Vec<f64>,
    pub luminosity: WeightLuminosity,
    pub turns: u64,
}

impl WeightState {
    /// Create from the three models' current weights.
    pub fn from_models(surface: &Surface, shatter: &Shatter, reflection: &Reflection) -> Self {
        WeightState {
            surface_w1: surface.w1.clone(),
            surface_b1: surface.b1.clone(),
            surface_w2: surface.w2.clone(),
            surface_b2: surface.b2.clone(),

            shatter_w1: shatter.w1.clone(),
            shatter_b1: shatter.b1.clone(),
            shatter_w2: shatter.w2.clone(),
            shatter_b2: shatter.b2.clone(),
            shatter_concept_embed: shatter.concept_embed.clone(),
            shatter_slot_embed: shatter.slot_embed.clone(),

            reflection_w1: reflection.w1.clone(),
            reflection_b1: reflection.b1.clone(),
            reflection_w2: reflection.w2.clone(),
            reflection_b2: reflection.b2.clone(),

            eigenvalues: Vec::new(),
            luminosity: WeightLuminosity::Dark,
            turns: 0,
        }
    }

    /// Apply weights back to the three models.
    pub fn apply_to_models(&self, surface: &mut Surface, shatter: &mut Shatter, reflection: &mut Reflection) {
        surface.w1 = self.surface_w1.clone();
        surface.b1 = self.surface_b1.clone();
        surface.w2 = self.surface_w2.clone();
        surface.b2 = self.surface_b2.clone();

        shatter.w1 = self.shatter_w1.clone();
        shatter.b1 = self.shatter_b1.clone();
        shatter.w2 = self.shatter_w2.clone();
        shatter.b2 = self.shatter_b2.clone();
        shatter.concept_embed = self.shatter_concept_embed.clone();
        shatter.slot_embed = self.shatter_slot_embed.clone();

        reflection.w1 = self.reflection_w1.clone();
        reflection.b1 = self.reflection_b1.clone();
        reflection.w2 = self.reflection_w2.clone();
        reflection.b2 = self.reflection_b2.clone();
    }

    /// Serialize to raw bytes (f64 little-endian).
    ///
    /// Format:
    /// - 8 bytes: magic "SHATTER\0"
    /// - 8 bytes: version (1u64 LE)
    /// - 8 bytes: turns (u64 LE)
    /// - 8 bytes: luminosity tag (0=Light, 1=Dimmed, 2=Dark)
    /// - 8 bytes: luminosity value (f64, 0.0 for Light/Dark)
    /// - 8 bytes: eigenvalue count (u64 LE)
    /// - eigenvalue_count * 8 bytes: eigenvalues (f64 LE)
    /// - Then: surface_w1, surface_b1, surface_w2, surface_b2
    /// -        shatter_w1, shatter_b1, shatter_w2, shatter_b2,
    /// -        shatter_concept_embed, shatter_slot_embed
    /// -        reflection_w1, reflection_b1, reflection_w2, reflection_b2
    /// Each preceded by length (u64 LE).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Magic
        buf.extend_from_slice(b"SHATTER\0");

        // Version
        buf.extend_from_slice(&1u64.to_le_bytes());

        // Turns
        buf.extend_from_slice(&self.turns.to_le_bytes());

        // Luminosity
        match &self.luminosity {
            WeightLuminosity::Light => {
                buf.extend_from_slice(&0u64.to_le_bytes());
                buf.extend_from_slice(&0.0f64.to_le_bytes());
            }
            WeightLuminosity::Dimmed(v) => {
                buf.extend_from_slice(&1u64.to_le_bytes());
                buf.extend_from_slice(&v.to_le_bytes());
            }
            WeightLuminosity::Dark => {
                buf.extend_from_slice(&2u64.to_le_bytes());
                buf.extend_from_slice(&0.0f64.to_le_bytes());
            }
        }

        // Eigenvalues
        write_vec(&mut buf, &self.eigenvalues);

        // Surface
        write_vec(&mut buf, &self.surface_w1);
        write_vec(&mut buf, &self.surface_b1);
        write_vec(&mut buf, &self.surface_w2);
        write_vec(&mut buf, &self.surface_b2);

        // Shatter
        write_vec(&mut buf, &self.shatter_w1);
        write_vec(&mut buf, &self.shatter_b1);
        write_vec(&mut buf, &self.shatter_w2);
        write_vec(&mut buf, &self.shatter_b2);
        write_vec(&mut buf, &self.shatter_concept_embed);
        write_vec(&mut buf, &self.shatter_slot_embed);

        // Reflection
        write_vec(&mut buf, &self.reflection_w1);
        write_vec(&mut buf, &self.reflection_b1);
        write_vec(&mut buf, &self.reflection_w2);
        write_vec(&mut buf, &self.reflection_b2);

        buf
    }

    /// Deserialize from raw bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        let mut offset = 0;

        // Magic
        if bytes.len() < 8 || &bytes[0..8] != b"SHATTER\0" {
            return Err("invalid magic: expected SHATTER\\0".to_string());
        }
        offset += 8;

        // Version
        let version = read_u64(bytes, &mut offset)?;
        if version != 1 {
            return Err(format!("unsupported version: {}", version));
        }

        // Turns
        let turns = read_u64(bytes, &mut offset)?;

        // Luminosity
        let lum_tag = read_u64(bytes, &mut offset)?;
        let lum_val = read_f64(bytes, &mut offset)?;
        let luminosity = match lum_tag {
            0 => WeightLuminosity::Light,
            1 => WeightLuminosity::Dimmed(lum_val),
            2 => WeightLuminosity::Dark,
            _ => return Err(format!("invalid luminosity tag: {}", lum_tag)),
        };

        // Eigenvalues
        let eigenvalues = read_vec(bytes, &mut offset)?;

        // Surface
        let surface_w1 = read_vec(bytes, &mut offset)?;
        let surface_b1 = read_vec(bytes, &mut offset)?;
        let surface_w2 = read_vec(bytes, &mut offset)?;
        let surface_b2 = read_vec(bytes, &mut offset)?;

        // Shatter
        let shatter_w1 = read_vec(bytes, &mut offset)?;
        let shatter_b1 = read_vec(bytes, &mut offset)?;
        let shatter_w2 = read_vec(bytes, &mut offset)?;
        let shatter_b2 = read_vec(bytes, &mut offset)?;
        let shatter_concept_embed = read_vec(bytes, &mut offset)?;
        let shatter_slot_embed = read_vec(bytes, &mut offset)?;

        // Reflection
        let reflection_w1 = read_vec(bytes, &mut offset)?;
        let reflection_b1 = read_vec(bytes, &mut offset)?;
        let reflection_w2 = read_vec(bytes, &mut offset)?;
        let reflection_b2 = read_vec(bytes, &mut offset)?;

        Ok(WeightState {
            surface_w1,
            surface_b1,
            surface_w2,
            surface_b2,
            shatter_w1,
            shatter_b1,
            shatter_w2,
            shatter_b2,
            shatter_concept_embed,
            shatter_slot_embed,
            reflection_w1,
            reflection_b1,
            reflection_w2,
            reflection_b2,
            eigenvalues,
            luminosity,
            turns,
        })
    }

    /// Save to disk.
    pub fn save(&self, path: &str) -> Result<(), String> {
        let bytes = self.to_bytes();
        std::fs::write(path, bytes).map_err(|e| format!("save '{}': {}", path, e))
    }

    /// Load from disk.
    pub fn load(path: &str) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("load '{}': {}", path, e))?;
        Self::from_bytes(&bytes)
    }
}

// ---------------------------------------------------------------------------
// Serialization helpers
// ---------------------------------------------------------------------------

fn write_vec(buf: &mut Vec<u8>, v: &[f64]) {
    buf.extend_from_slice(&(v.len() as u64).to_le_bytes());
    for &val in v {
        buf.extend_from_slice(&val.to_le_bytes());
    }
}

fn read_u64(bytes: &[u8], offset: &mut usize) -> Result<u64, String> {
    if *offset + 8 > bytes.len() {
        return Err("unexpected end of data reading u64".to_string());
    }
    let val = u64::from_le_bytes(bytes[*offset..*offset + 8].try_into().unwrap());
    *offset += 8;
    Ok(val)
}

fn read_f64(bytes: &[u8], offset: &mut usize) -> Result<f64, String> {
    if *offset + 8 > bytes.len() {
        return Err("unexpected end of data reading f64".to_string());
    }
    let val = f64::from_le_bytes(bytes[*offset..*offset + 8].try_into().unwrap());
    *offset += 8;
    Ok(val)
}

fn read_vec(bytes: &[u8], offset: &mut usize) -> Result<Vec<f64>, String> {
    let len = read_u64(bytes, offset)? as usize;
    let byte_len = len * 8;
    if *offset + byte_len > bytes.len() {
        return Err("unexpected end of data reading vec".to_string());
    }
    let mut v = Vec::with_capacity(len);
    for i in 0..len {
        let start = *offset + i * 8;
        let val = f64::from_le_bytes(bytes[start..start + 8].try_into().unwrap());
        v.push(val);
    }
    *offset += byte_len;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_weight_state() -> WeightState {
        let surface = Surface::untrained(42);
        let shatter = Shatter::untrained(42);
        let reflection = Reflection::untrained(42);
        let mut ws = WeightState::from_models(&surface, &shatter, &reflection);
        ws.eigenvalues = vec![0.5, 0.3, 0.8];
        ws.luminosity = WeightLuminosity::Dimmed(0.23);
        ws.turns = 47;
        ws
    }

    #[test]
    fn weight_state_roundtrip_bytes() {
        let ws = make_weight_state();
        let bytes = ws.to_bytes();
        let ws2 = WeightState::from_bytes(&bytes).expect("from_bytes failed");
        assert_eq!(ws, ws2);
    }

    #[test]
    fn weight_state_save_load() {
        let ws = make_weight_state();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.shatter");
        let path_str = path.to_str().unwrap();

        ws.save(path_str).expect("save failed");
        let ws2 = WeightState::load(path_str).expect("load failed");
        assert_eq!(ws, ws2);
    }

    #[test]
    fn weight_state_from_models_roundtrip() {
        let surface = Surface::untrained(42);
        let shatter = Shatter::untrained(42);
        let reflection = Reflection::untrained(42);

        let ws = WeightState::from_models(&surface, &shatter, &reflection);

        let mut surface2 = Surface::untrained(99);
        let mut shatter2 = Shatter::untrained(99);
        let mut reflection2 = Reflection::untrained(99);

        ws.apply_to_models(&mut surface2, &mut shatter2, &mut reflection2);

        assert_eq!(surface.w1, surface2.w1);
        assert_eq!(surface.b1, surface2.b1);
        assert_eq!(shatter.w1, shatter2.w1);
        assert_eq!(reflection.w1, reflection2.w1);
    }

    #[test]
    fn weight_state_luminosity_variants() {
        // Light
        let mut ws = make_weight_state();
        ws.luminosity = WeightLuminosity::Light;
        let bytes = ws.to_bytes();
        let ws2 = WeightState::from_bytes(&bytes).unwrap();
        assert_eq!(ws2.luminosity, WeightLuminosity::Light);

        // Dark
        ws.luminosity = WeightLuminosity::Dark;
        let bytes = ws.to_bytes();
        let ws2 = WeightState::from_bytes(&bytes).unwrap();
        assert_eq!(ws2.luminosity, WeightLuminosity::Dark);

        // Dimmed
        ws.luminosity = WeightLuminosity::Dimmed(0.42);
        let bytes = ws.to_bytes();
        let ws2 = WeightState::from_bytes(&bytes).unwrap();
        assert_eq!(ws2.luminosity, WeightLuminosity::Dimmed(0.42));
    }

    #[test]
    fn weight_state_invalid_magic() {
        let result = WeightState::from_bytes(b"INVALID\0");
        assert!(result.is_err());
    }

    #[test]
    fn weight_state_empty_bytes() {
        let result = WeightState::from_bytes(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn weight_state_turns_persisted() {
        let mut ws = make_weight_state();
        ws.turns = 123;
        let bytes = ws.to_bytes();
        let ws2 = WeightState::from_bytes(&bytes).unwrap();
        assert_eq!(ws2.turns, 123);
    }

    #[test]
    fn weight_state_eigenvalues_persisted() {
        let mut ws = make_weight_state();
        ws.eigenvalues = vec![0.1, 0.2, 0.3, 0.4, 0.5];
        let bytes = ws.to_bytes();
        let ws2 = WeightState::from_bytes(&bytes).unwrap();
        assert_eq!(ws2.eigenvalues, vec![0.1, 0.2, 0.3, 0.4, 0.5]);
    }
}
