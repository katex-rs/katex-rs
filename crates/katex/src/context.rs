//! Global context for various operations

use crate::FontMetricsData;
use crate::font_metrics::MetricMap;
use crate::namespace::KeyMap;

use crate::{
    define_environment,
    define_environment::{EnvDefSpec, EnvSpec},
    define_function::{FunctionDefSpec, FunctionSpec, HtmlBuilder, MathMLBuilder},
    font_metrics::{FONT_METRICS, FontMetrics, FontSizeIndex},
    functions,
    parser::parse_node::NodeType,
    symbols::{Symbols, create_symbols},
};

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::wasm_bindgen;

/// Global context struct for KaTeX
///
/// The KatexContext serves as the central registry for all KaTeX functionality,
/// containing mappings for functions, HTML/MathML builders, symbols, and
/// environments. This design enables runtime extensibility of LaTeX commands
/// without recompilation.
#[cfg_attr(feature = "wasm", wasm_bindgen)]
pub struct KatexContext {
    /// Corresponds to _functions in defineFunction.js
    /// All registered functions
    /// `Parser` requires this dictionary
    #[cfg_attr(feature = "wasm", wasm_bindgen(skip))]
    pub functions: KeyMap<String, FunctionSpec>,
    /// Corresponds to _htmlGroupBuilders in defineFunction.js
    /// All HTML builders. Should be only used in the `define*` and the
    /// `build*ML` functions.
    #[cfg_attr(feature = "wasm", wasm_bindgen(skip))]
    pub html_group_builders: KeyMap<NodeType, HtmlBuilder>,
    /// Corresponds to _mathmlGroupBuilders in defineFunction.js
    /// All MathML builders. Should be only used in the `define*` and the
    /// `build*ML` functions.
    #[cfg_attr(feature = "wasm", wasm_bindgen(skip))]
    pub mathml_group_builders: KeyMap<NodeType, MathMLBuilder>,
    /// Corresponds to symbols in symbols.js
    /// Main symbol table mapping modes to character information
    #[cfg_attr(feature = "wasm", wasm_bindgen(skip))]
    pub symbols: Symbols,
    /// Corresponds to _environments in defineEnvironment.js
    /// All registered environments.
    #[cfg_attr(feature = "wasm", wasm_bindgen(skip))]
    pub environments: KeyMap<String, EnvSpec>,
    /// Font metrics data for character measurements
    #[cfg_attr(feature = "wasm", wasm_bindgen(skip))]
    pub font_metrics: FontMetricsData,
}

impl KatexContext {
    /// Set default values of functions
    pub fn define_function(&mut self, spec: FunctionDefSpec) {
        let data = FunctionSpec {
            node_type: spec.node_type,
            num_args: spec.props.num_args,
            arg_types: spec.props.arg_types,
            allowed_in_argument: spec.props.allowed_in_argument,
            allowed_in_text: spec.props.allowed_in_text,
            allowed_in_math: spec.props.allowed_in_math,
            num_optional_args: spec.props.num_optional_args,
            infix: spec.props.infix,
            primitive: spec.props.primitive,
            handler: spec.handler,
        };

        // Register function names
        for name in spec.names {
            self.functions.insert((*name).to_owned(), data.clone());
        }

        // Register builders if type is specified
        if let Some(node_type) = spec.node_type {
            self.define_function_builders(node_type, spec.html_builder, spec.mathml_builder);
        }
    }

    /// Register only the HTML and MathML builders for a function
    pub fn define_function_builders(
        &mut self,
        node_type: NodeType,
        html_builder: Option<HtmlBuilder>,
        mathml_builder: Option<MathMLBuilder>,
    ) {
        if let Some(builder) = html_builder {
            self.html_group_builders.insert(node_type, builder);
        }

        if let Some(builder) = mathml_builder {
            self.mathml_group_builders.insert(node_type, builder);
        }
    }

    /// Set default values of environments
    pub fn define_environment(&mut self, spec: EnvDefSpec) {
        let data = EnvSpec {
            node_type: spec.node_type,
            num_args: spec.props.num_args.unwrap_or(0),
            arg_types: spec.props.arg_types.clone(),
            allowed_in_text: spec.props.allowed_in_text.unwrap_or(false),
            num_optional_args: spec.props.num_optional_args.unwrap_or(0),
            handler: spec.handler,
        };

        // Register environment names
        for name in spec.names {
            self.environments.insert(name, data.clone());
        }

        if let Some(html_builder) = spec.html_builder {
            self.html_group_builders
                .insert(spec.node_type, html_builder);
        }

        if let Some(mathml_builder) = spec.mathml_builder {
            self.mathml_group_builders
                .insert(spec.node_type, mathml_builder);
        }
    }

    /// Get the font metrics for a given size.
    #[must_use]
    pub fn get_global_metrics(&self, size: f64) -> &FontMetrics {
        let size_index: FontSizeIndex = if size >= 5.0 {
            0
        } else if size >= 3.0 {
            1
        } else {
            2
        };

        &FONT_METRICS[size_index as usize]
    }

    /// Set or override font metrics for a specific font family
    pub fn set_font_metrics(&mut self, font_name: &str, metrics: MetricMap) {
        self.font_metrics
            .custom
            .insert(font_name.to_owned(), metrics);
    }
}

impl Default for KatexContext {
    fn default() -> Self {
        let mut ctx = Self {
            functions: KeyMap::default(),
            html_group_builders: KeyMap::default(),
            mathml_group_builders: KeyMap::default(),
            symbols: create_symbols(),
            environments: KeyMap::default(),
            font_metrics: FontMetricsData::default(),
        };
        // Register internal functions and symbols here if needed
        functions::define_relax(&mut ctx);
        functions::define_genfrac(&mut ctx);
        functions::define_accent(&mut ctx);
        functions::define_accentunder(&mut ctx);
        functions::define_arrow(&mut ctx);
        functions::define_char(&mut ctx);
        functions::define_color(&mut ctx);
        functions::define_cr(&mut ctx);
        functions::define_def(&mut ctx);
        functions::define_delimsizing(&mut ctx);
        functions::define_enclose(&mut ctx);
        functions::define_environment(&mut ctx);
        functions::define_genfrac(&mut ctx);
        functions::define_hbox(&mut ctx);
        functions::define_horiz_brace(&mut ctx);
        functions::define_href(&mut ctx);
        functions::define_html(&mut ctx);
        functions::define_htmlmathml(&mut ctx);
        functions::define_includegraphics(&mut ctx);
        functions::define_kern(&mut ctx);
        functions::define_lap(&mut ctx);
        functions::define_leftright(&mut ctx);
        functions::define_math(&mut ctx);
        functions::define_mathchoice(&mut ctx);
        functions::define_mclass(&mut ctx);
        functions::define_middle(&mut ctx);
        functions::define_ordgroup(&mut ctx);
        functions::define_overline(&mut ctx);
        functions::define_phantom(&mut ctx);
        functions::define_raisebox(&mut ctx);
        functions::define_rule(&mut ctx);
        functions::define_sizing(&mut ctx);
        functions::define_smash(&mut ctx);
        functions::define_spacing(&mut ctx);
        functions::define_sqrt(&mut ctx);
        functions::define_styling(&mut ctx);
        functions::define_supsub(&mut ctx);
        functions::define_symbols_op(&mut ctx);
        functions::define_symbols_ord(&mut ctx);
        functions::define_tag(&mut ctx);
        functions::define_text(&mut ctx);
        functions::define_underline(&mut ctx);
        functions::define_vcenter(&mut ctx);
        functions::define_verb(&mut ctx);
        functions::define_pmb(&mut ctx);
        functions::define_font(&mut ctx);
        functions::define_op(&mut ctx);
        functions::define_operatorname(&mut ctx);

        // Register environments
        define_environment::define_array(&mut ctx);
        define_environment::define_cd(&mut ctx);
        ctx
    }
}
