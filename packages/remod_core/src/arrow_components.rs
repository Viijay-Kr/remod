use std::path::PathBuf;

use remod_config::Config;
use swc_common::sync::Lrc;
use swc_common::{comments::SingleThreadedComments, SourceMap};
use swc_common::{Loc, Spanned};
use swc_ecma_ast::{
    ArrowExpr, BindingIdent, BlockStmt, BlockStmtOrExpr, Expr, FnExpr, Module, Pat, Program, Stmt,
    VarDeclarator,
};
use swc_ecma_visit::{Visit, VisitWith};

use crate::utils::{is_jsx_like, parse_module};

#[allow(unused)]
pub struct ArrowExpressionComponents {
    pub variables: Vec<BindingIdent>,
    module: Module,
    cm: Lrc<SourceMap>,
    comments: SingleThreadedComments,
}

impl ArrowExpressionComponents {
    pub fn new(path: &PathBuf, config: &Config) -> Self {
        let (module, cm, comments) = parse_module(path, config);
        ArrowExpressionComponents {
            variables: vec![],
            cm,
            comments,
            module,
        }
    }
    fn process_arrow_expression(&mut self, expr: ArrowExpr, n: &VarDeclarator) {
        match *expr.body {
            BlockStmtOrExpr::BlockStmt(block) => self.process_block_statements(block, n),
            BlockStmtOrExpr::Expr(expr) => {
                if is_jsx_like(&expr) {
                    match n.name {
                        Pat::Ident(ref id) => self.variables.push(id.to_owned()),
                        _ => {}
                    }
                }
            }
        }
    }

    fn process_block_statements(&mut self, block: BlockStmt, n: &VarDeclarator) {
        for stmt in block.stmts.iter() {
            match stmt {
                Stmt::Return(ret) => match &ret.arg {
                    Some(arg) => {
                        if is_jsx_like(arg) {
                            match n.name.clone() {
                                Pat::Ident(id) => self.variables.push(id),
                                _ => {}
                            }
                        }
                    }
                    None => {}
                },
                _ => {}
            }
        }
    }
    fn process_func_expression(&mut self, expr: FnExpr, n: &VarDeclarator) {
        match expr.function.body {
            Some(stmt) => self.process_block_statements(stmt, n),
            None => {}
        }
    }
    pub fn extract_components(&mut self) {
        let program = Program::Module(self.module.to_owned());
        program.visit_with(self);
    }

    pub fn look_up_variable(&self, v: &BindingIdent) -> (Loc, Loc) {
        let start = self.cm.lookup_char_pos(v.span.lo());
        let end = self.cm.lookup_char_pos(v.span_hi());
        (start, end)
    }
}

impl Visit for ArrowExpressionComponents {
    fn visit_var_declarator(&mut self, n: &VarDeclarator) {
        match &n.init {
            Some(expr) => match *expr.to_owned() {
                Expr::Arrow(exp) => self.process_arrow_expression(exp, n),
                Expr::Call(call) => {
                    if call.args.len() > 0 {
                        let arg = &call.args[0];
                        match *arg.expr.to_owned() {
                            Expr::Fn(expr) => self.process_func_expression(expr, n),
                            Expr::Arrow(expr) => self.process_arrow_expression(expr, n),
                            _ => {}
                        }
                    }
                }
                _ => {}
            },
            None => {}
        }
    }
}
