use std::path::PathBuf;

use glob::Pattern;
use remod_config::Config;
use swc_common::comments::SingleThreadedComments;
use swc_common::sync::Lrc;
use swc_common::{
    errors::{ColorConfig, Handler},
    SourceMap,
};
use swc_ecma_ast::Module;
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};
use swc_ecma_parser::{EsConfig, TsConfig};

pub fn should_ignore_entry(ignore: &Vec<String>, path: &PathBuf) -> bool {
    ignore.iter().any(|it| {
        let pattern = Pattern::new(it);
        match pattern {
            Ok(p) => p.matches_path(path.as_path()),
            Err(..) => {
                println!("Ignore Pattern match failed for {}", it);
                false
            }
        }
    })
}

pub fn get_module(
    path: &PathBuf,
    config: &Config,
) -> (Module, Lrc<SourceMap>, SingleThreadedComments) {
    let cm: Lrc<SourceMap> = Default::default();
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));
    let fm = cm.load_file(path.as_path()).expect("failed to load ");
    let ts_config = TsConfig {
        tsx: true,
        disallow_ambiguous_jsx_like: false,
        ..Default::default()
    };
    let syntax = if config.typescript.is_some() {
        Syntax::Typescript(ts_config)
    } else {
        Syntax::Es(EsConfig {
            jsx: true,
            ..Default::default()
        })
    };
    let comments: SingleThreadedComments = SingleThreadedComments::default();
    let lexer = Lexer::new(
        syntax,
        // EsVersion defaults to es5
        Default::default(),
        StringInput::from(&*fm),
        Some(&comments),
    );

    let mut parser = Parser::new_from(lexer);

    for e in parser.take_errors() {
        e.into_diagnostic(&handler).emit();
    }

    let mut _module = parser.parse_module().expect("failed to parser module");
    (_module, cm, comments)
}
