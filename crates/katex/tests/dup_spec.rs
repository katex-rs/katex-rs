#![allow(clippy::non_ascii_literal)]

mod setup;

use katex::{
    macros::builtins::BUILTIN_MACROS,
    symbols::{Mode as SymbolMode, create_symbols},
};
use setup::*;

#[test]
fn symbols_and_macros() {
    it("macros should not shadow a symbol", || {
        let symbols = create_symbols();
        for (macro_name, _) in BUILTIN_MACROS.entries() {
            assert!(
                !symbols.contains(SymbolMode::Math, macro_name)
                    && !symbols.contains(SymbolMode::Text, macro_name),
                "Macro {macro_name} should not shadow a symbol",
            );
        }
        Ok(())
    });
}
