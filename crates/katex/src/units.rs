//! Units conversion utilities (Rust port of KaTeX src/units.js)
//!
//! This module performs conversion between TeX/KaTeX units and CSS ems.
//! It provides helpers similar to the original JavaScript implementation:
//! - `valid_unit` to validate a unit string or measurement
//! - `calculate_size` to convert a `Measurement` into ems for the given
//!   `Options`
//! - `make_em` to format a number as an em string rounded to 4 decimals

use crate::KatexContext;
use crate::options::Options;
use crate::spacing_data::Measurement;
use crate::types::{ParseError, ParseErrorKind};

/// Return TeX points per unit for absolute TeX units.
/// See KaTeX src/units.js `ptPerUnit` for reference values.
fn pt_per_unit<T>(unit: &T) -> Option<f64>
where
    T: AsRef<str>,
{
    match unit.as_ref() {
        // https://en.wikibooks.org/wiki/LaTeX/Lengths
        // https://tex.stackexchange.com/a/8263
        "pt" => Some(1.0),             // TeX point
        "mm" => Some(7227.0 / 2540.0), // millimeter
        "cm" => Some(7227.0 / 254.0),  // centimeter
        "in" => Some(72.27),           // inch
        // https://tex.stackexchange.com/a/41371
        "bp" | "px" => Some(803.0 / 800.0), // big (PostScript) points
        // \pdfpxdimen defaults to 1 bp in pdfTeX and LuaTeX
        "pc" => Some(12.0),             // pica
        "dd" => Some(1238.0 / 1157.0),  // didot
        "cc" => Some(14856.0 / 1157.0), // cicero (12 didot)
        "nd" => Some(685.0 / 642.0),    // new didot
        "nc" => Some(1370.0 / 107.0),   // new cicero (12 new didot)
        "sp" => Some(1.0 / 65536.0),    // scaled point (TeX's internal smallest unit)
        _ => None,
    }
}

/// Check whether a unit string is a valid length unit understood by KaTeX.
pub fn valid_unit_str<T>(unit: T) -> bool
where
    T: AsRef<str>,
{
    pt_per_unit(&unit).is_some() || matches!(unit.as_ref(), "ex" | "em" | "mu")
}

/// Check whether a measurement has a valid unit.
pub fn valid_unit<T>(measurement: &Measurement<T>) -> bool
where
    T: AsRef<str>,
{
    valid_unit_str(&measurement.unit)
}

impl KatexContext {
    /// Convert a `Measurement` (e.g., `{ number: 1.2, unit: "cm" }`) into CSS
    /// ems for the given `Options`. Mirrors the logic in KaTeX
    /// `calculateSize`.
    ///
    /// Returns an error if the unit is invalid.
    pub fn calculate_size<T>(
        &self,
        size: &Measurement<T>,
        options: &Options,
    ) -> Result<f64, ParseError>
    where
        T: AsRef<str>,
    {
        let mut scale: f64;

        if let Some(pt) = pt_per_unit(&size.unit) {
            // Absolute units. Convert unit -> pt -> em, then unscale absolute to current
            // size.
            let metrics = self.get_global_metrics(options.size as f64);
            let pt_per_em = metrics.pt_per_em;
            scale = pt / pt_per_em / options.size_multiplier;
        } else if size.unit.as_ref() == "mu" {
            // `mu` units scale with scriptstyle/scriptscriptstyle.
            let metrics = self.get_global_metrics(options.size as f64);
            scale = metrics.css_em_per_mu;
        } else {
            // Other relative units always refer to the textstyle font in the current size.
            let unit_options = if options.style.is_tight() {
                options.having_style(options.style.text())
            } else {
                options.clone()
            };

            let metrics = self.get_global_metrics(unit_options.size as f64);
            scale = match size.unit.as_ref() {
                "ex" => metrics.x_height,
                "em" => metrics.quad,
                other => {
                    return Err(ParseError::new(ParseErrorKind::InvalidUnit {
                        unit: other.to_owned(),
                    }));
                }
            };

            // If we changed options for tight style, compensate for size multiplier.
            if unit_options.size != options.size {
                let ratio = unit_options.size_multiplier / options.size_multiplier;
                scale *= ratio;
            }
        }

        Ok(f64::min(size.number * scale, options.max_size))
    }
}

/// Round to 4 decimal places and append "em", dropping trailing zeros.
#[must_use]
pub fn make_em(n: f64) -> String {
    // Format with 4 decimals like JavaScript's `Number#toFixed(4)`
    let mut s = format!("{n:.4}");

    if s.contains('.') {
        while s.ends_with('0') {
            s.pop();
        }
        if s.ends_with('.') {
            s.pop();
        }
    }

    if s == "-0" {
        "0".clone_into(&mut s);
    } else if s.is_empty() {
        s.push('0');
    }

    s.push_str("em");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::Options;
    use crate::spacing_data::MeasurementOwned;
    use crate::style;

    fn default_options() -> Options {
        Options::builder()
            .style(style::TEXT)
            .max_size(1_000_000.0)
            .min_rule_thickness(0.04)
            .build()
    }

    #[test]
    fn test_valid_unit() {
        assert!(valid_unit_str("pt"));
        assert!(valid_unit_str("cm"));
        assert!(valid_unit_str("px"));
        assert!(valid_unit_str("em"));
        assert!(valid_unit_str("ex"));
        assert!(valid_unit_str("mu"));
        assert!(!valid_unit_str("bogus"));
    }

    #[test]
    fn test_make_em_rounding() {
        assert_eq!(make_em(1.0), "1em");
        assert_eq!(make_em(1.23456), "1.2346em");
        assert_eq!(make_em(0.00004), "0em");
        assert_eq!(make_em(0.00005), "0.0001em");
    }

    #[test]
    fn test_calculate_size_absolute_units() {
        let opts = default_options();
        let ctx = KatexContext::default();
        // 10pt per em by default, 1pt = 1/10 em
        let m = MeasurementOwned {
            number: 10.0,
            unit: "pt".to_owned(),
        };
        let ems = ctx.calculate_size(&m, &opts).unwrap();
        assert!((ems - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_calculate_size_relative_units() {
        let opts = default_options();
        let ctx = KatexContext::default();
        // In text style, quad is 1em
        let m_em = MeasurementOwned {
            number: 2.0,
            unit: "em".to_owned(),
        };
        let ems_em = ctx.calculate_size(&m_em, &opts).unwrap();
        assert!((ems_em - 2.0).abs() < 1e-9);

        // xHeight default is 0.431em in text
        let measure_owned = MeasurementOwned {
            number: 1.0,
            unit: "ex".to_owned(),
        };
        let ems_owned = ctx.calculate_size(&measure_owned, &opts).unwrap();
        assert!((ems_owned - 0.431).abs() < 1e-9);
    }
}
