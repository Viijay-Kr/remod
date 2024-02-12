use std::{
    fs::{File, OpenOptions},
    io::{prelude::*, BufReader, Write},
};

use glob::Paths;
use remod_config::Config;
use swc_atoms::Atom;
use swc_common::{chain, util::take::Take, SourceFileAndLine, Spanned};
use swc_ecma_ast::{
    ArrowExpr, AssignExpr, BindingIdent, BlockStmt, BlockStmtOrExpr, Expr, ExprStmt, FnExpr, Ident,
    Lit, MemberExpr, MemberProp, ModuleItem, Pat, PatOrExpr, Program, Stmt, VarDeclarator,
};
use swc_ecma_visit::{Visit, VisitMut, VisitMutWith, VisitWith};

use crate::utils::{parse_module, should_ignore_entry};

pub struct VariableDeclarationWalker {
    pub variables: Vec<BindingIdent>,
}

impl VariableDeclarationWalker {
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
}

impl Visit for VariableDeclarationWalker {
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

pub struct FunctionDelarationWalker {
    pub function_decls: Vec<Ident>,
}
impl Visit for FunctionDelarationWalker {
    fn visit_fn_decl(&mut self, func: &swc_ecma_ast::FnDecl) {
        match &func.function.body {
            Some(body) => {
                for stmt in body.stmts.iter() {
                    match stmt {
                        swc_ecma_ast::Stmt::Return(ret) => match &ret.arg {
                            Some(expr) => {
                                if is_jsx_like(expr) {
                                    self.function_decls.push(func.ident.clone());
                                }
                            }
                            None => todo!(),
                        },
                        _ => {}
                    }
                }
            }
            None => {}
        }
    }
}

struct ExpressionStatementVisitor {
    assignment_expressions: Vec<MemberExpr>,
    expressions: Vec<Box<Expr>>,
}
impl Visit for ExpressionStatementVisitor {
    fn visit_expr_stmt(&mut self, n: &ExprStmt) {
        match *n.expr {
            Expr::Assign(ref expr) => match &expr.left {
                PatOrExpr::Pat(ref pat) => match **pat {
                    Pat::Expr(ref expr) => match **expr {
                        Expr::Member(ref mem) => match mem.prop {
                            MemberProp::Ident(ref id) => {
                                if &*id.sym == "displayName" {
                                    self.assignment_expressions.push(mem.to_owned());
                                    self.expressions.push(n.expr.to_owned())
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
}

struct ExpressionStatementMutVisitor {}
impl VisitMut for ExpressionStatementMutVisitor {
    fn visit_mut_expr_stmt(&mut self, n: &mut ExprStmt) {
        match *n.expr {
            Expr::Assign(ref expr) => match &expr.left {
                PatOrExpr::Pat(ref pat) => match **pat {
                    Pat::Expr(ref expr) => match **expr {
                        Expr::Member(ref mem) => match mem.prop {
                            MemberProp::Ident(ref id) => {
                                if &*id.sym == "displayName" {
                                    n.expr.take();
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            },

            _ => {}
        }
    }

    fn visit_mut_stmt(&mut self, n: &mut Stmt) {
        n.visit_mut_children_with(self);
        match n {
            Stmt::Expr(expr) => {
                if expr.expr.is_invalid() {
                    n.take();
                }
            }
            _ => {}
        }
    }
    fn visit_mut_stmts(&mut self, n: &mut Vec<Stmt>) {
        n.visit_mut_children_with(self);
        n.retain(|node| !matches!(node, Stmt::Empty(..)));
    }
    fn visit_mut_module_items(&mut self, n: &mut Vec<swc_ecma_ast::ModuleItem>) {
        n.visit_mut_children_with(self);
        n.retain(|node| !matches!(node, ModuleItem::Stmt(Stmt::Empty(..))));
    }
}

pub struct RenameDisplayNameVisitor {
    pub assign_exprs: Vec<(AssignExpr, String)>,
    prefix: String,
}
impl Visit for RenameDisplayNameVisitor {
    fn visit_expr_stmt(&mut self, n: &ExprStmt) {
        match *n.expr {
            Expr::Assign(ref expr) => match &expr.left {
                PatOrExpr::Pat(ref pat) => match **pat {
                    Pat::Expr(ref pat_expr) => match **pat_expr {
                        Expr::Member(ref mem) => match *mem.obj {
                            Expr::Ident(ref obj) => match mem.prop {
                                MemberProp::Ident(ref ident) => {
                                    if ident.sym.as_str() == "displayName" {
                                        match *expr.right.to_owned() {
                                            Expr::Lit(lit) => match lit {
                                                Lit::Str(..) => {
                                                    self.assign_exprs.push((
                                                        expr.to_owned(),
                                                        format!(
                                                            "{}.displayName = \"{}_{}\"",
                                                            obj.sym, self.prefix, obj.sym
                                                        ),
                                                    ));
                                                }
                                                _ => {}
                                            },
                                            _ => {}
                                        }
                                    }
                                }
                                _ => {}
                            },
                            _ => {}
                        },
                        _ => {}
                    },
                    _ => {}
                },
                _ => {}
            },
            _ => {}
        }
    }
}

fn is_jsx_like(expr: &Box<Expr>) -> bool {
    match *expr.to_owned() {
        Expr::JSXMember(..)
        | Expr::JSXNamespacedName(..)
        | Expr::JSXEmpty(..)
        | Expr::JSXElement(..)
        | Expr::JSXFragment(..) => true,
        Expr::Paren(exp) => is_jsx_like(&exp.expr),
        _ => false,
    }
}

fn does_expression_exists(exp: &MemberExpr, sym: &Atom) -> bool {
    match *exp.obj {
        Expr::Ident(ref obj_ident) => match exp.prop {
            MemberProp::Ident(ref ident) => {
                if ident.sym.as_str() == "displayName" && obj_ident.sym.as_str() == sym.as_str() {
                    return true;
                } else {
                    return false;
                }
            }
            _ => false,
        },
        _ => false,
    }
}

#[derive(Debug, Clone, Default)]
pub struct DisplayName {
    /// List of statments to be processed
    pub stmts: Vec<String>,
    ///  Count of modified files
    pub modified: i64,
    /// Count of ignored files
    pub ignored: i64,
    /// Count of total files
    pub total: i64,
    /// Prefix for the display name property
    pub prefix: String,
}

impl DisplayName {
    fn create_stmts(&mut self, symbol: &Atom) -> String {
        let display_name = format!("{}.displayName = \"{}_{}\"", symbol, self.prefix, symbol);
        return display_name;
    }

    pub fn add_display_name(&mut self, files: Paths, config: &Config) {
        for entry in files {
            match entry {
                Ok(path) => {
                    self.total += 1;
                    if should_ignore_entry(&config.ignore, &path) {
                        println!("Skipping {}", path.display());
                        self.ignored += 1;
                        continue;
                    }

                    println!("{}", path.display());
                    let (mut _module, _cm, _comments) = parse_module(&path, &config);
                    let mut variable_decl_visitor = VariableDeclarationWalker { variables: vec![] };
                    let mut function_decl_visitor = FunctionDelarationWalker {
                        function_decls: vec![],
                    };
                    let mut expression_stmts_visitor = ExpressionStatementVisitor {
                        assignment_expressions: vec![],
                        expressions: vec![],
                    };
                    let mut visitor = chain!(
                        &mut variable_decl_visitor,
                        &mut function_decl_visitor,
                        &mut expression_stmts_visitor
                    );
                    let program = Program::Module(_module);
                    program.visit_with(&mut visitor);
                    let mut stmts: Vec<String> = vec![];
                    if expression_stmts_visitor.assignment_expressions.len() > 0 {
                        for variable in variable_decl_visitor.variables.clone() {
                            if expression_stmts_visitor
                                .assignment_expressions
                                .iter()
                                .find(|exp| does_expression_exists(exp, &variable.sym))
                                .is_some()
                            {
                                print!(
                                    "=> Display name already exists for the Component '{}'\n",
                                    variable.sym
                                );
                            } else {
                                stmts.push(self.create_stmts(&variable.sym));
                            }
                        }
                        for ident in function_decl_visitor.function_decls.clone() {
                            if expression_stmts_visitor
                                .assignment_expressions
                                .iter()
                                .find(|exp| does_expression_exists(exp, &ident.sym))
                                .is_some()
                            {
                                print!(
                                    "=> Display name already exists for the Component '{}' \n",
                                    ident.sym
                                );
                            } else {
                                stmts.push(self.create_stmts(&ident.sym));
                            }
                        }
                    } else {
                        for variable in variable_decl_visitor.variables.clone() {
                            stmts.push(self.create_stmts(&variable.sym));
                        }
                        for ident in function_decl_visitor.function_decls.clone() {
                            stmts.push(self.create_stmts(&ident.sym));
                        }
                    }
                    let file = OpenOptions::new().append(true).write(true).open(path);
                    match file {
                        Ok(mut f) => {
                            if stmts.len() > 0 {
                                let _ = f.write("\n".as_bytes());
                                let res = f.write(stmts.join("\n").as_bytes());
                                match res {
                                    Ok(_) => {
                                        self.modified += 1;
                                    }
                                    Err(e) => {
                                        println!("{:#?}", e);
                                    }
                                }
                            }
                        }
                        Err(err) => println!("{:?}", err),
                    }
                }
                Err(err) => println!("{:?}", err),
            }
        }
    }
    pub fn remove_display_name(&mut self, files: Paths, config: &Config) {
        for entry in files {
            match entry {
                Ok(path) => {
                    self.total += 1;
                    if should_ignore_entry(&config.ignore, &path) {
                        println!("Skipping {}", path.display());
                        self.ignored += 1;
                        continue;
                    }
                    println!("{}", path.display());
                    let (_module, _cm, _comments) = parse_module(&path, &config);
                    let program = Program::Module(_module);

                    let mut expr_vistor = ExpressionStatementVisitor {
                        assignment_expressions: vec![],
                        expressions: vec![],
                    };
                    // let mut visitor = chain!(expr_vistor,);
                    program.visit_with(&mut expr_vistor);

                    let file_reader = File::open(path.as_path());
                    match file_reader {
                        Ok(fr) => {
                            let reader = BufReader::new(fr);

                            let lines_to_remove = expr_vistor
                                .expressions
                                .iter()
                                .map(|stmt| {
                                    let start_line = _cm.lookup_line(stmt.span_lo()).unwrap();
                                    let end_line = _cm.lookup_line(stmt.span_hi()).unwrap();
                                    return (start_line, end_line);
                                })
                                .collect::<Vec<(SourceFileAndLine, SourceFileAndLine)>>();
                            let mut lines_to_keep: Vec<String> = Vec::new();
                            if lines_to_remove.len() > 0 {
                                for (line_num, may_be_line) in reader.lines().enumerate() {
                                    let line = may_be_line.unwrap();
                                    let existing_line =
                                        lines_to_remove.iter().find(|(st_line, end_line)| {
                                            st_line.line == line_num || end_line.line == line_num
                                        });

                                    if existing_line.is_some() {
                                        continue;
                                    }
                                    lines_to_keep.push(line)
                                }
                                let file = OpenOptions::new()
                                    .write(true)
                                    .truncate(true)
                                    .open(path.as_path());
                                match file {
                                    Ok(mut file) => {
                                        let res = file.write(lines_to_keep.join("\n").as_bytes());
                                        match res {
                                            Ok(_) => {
                                                self.modified += 1;
                                            }
                                            Err(e) => {
                                                println!("{:#?}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        println!("{:#?}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            println!("{:#?}", e);
                        }
                    }
                }
                Err(err) => println!("{:?}", err),
            }
        }
    }

    pub fn rename_display_names(&mut self, files: Paths, config: &Config) {
        for entry in files {
            match entry {
                Ok(path) => {
                    self.total += 1;
                    if should_ignore_entry(&config.ignore, &path) {
                        println!("Skipping {}", path.display());
                        self.ignored += 1;
                        continue;
                    } else {
                        println!("{}", path.display());
                        let (module, _cm, _comments) = parse_module(&path, &config);
                        let program = Program::Module(module);
                        let mut rename_visitor = RenameDisplayNameVisitor {
                            assign_exprs: vec![],
                            prefix: self.prefix.clone(),
                        };

                        program.visit_with(&mut rename_visitor);
                        let file_reader = File::open(path.as_path());
                        match file_reader {
                            Ok(fr) => {
                                let reader = BufReader::new(fr);
                                let lines_to_modify = rename_visitor
                                    .assign_exprs
                                    .iter()
                                    .map(|(expr, to_replace)| {
                                        let start_line = _cm
                                            .lookup_line(expr.span().lo)
                                            .unwrap();
                                        let end_line = _cm
                                            .lookup_line(expr.span().hi)
                                            .unwrap();
                                        return (
                                            start_line, end_line,
                                            to_replace,
                                        );
                                    })
                                    .collect::<Vec<(
                                        SourceFileAndLine,
                                        SourceFileAndLine,
                                        &String,
                                    )>>(
                                    );
                                let mut new_lines: Vec<String> = Vec::new();
                                for (line_num, may_be_line) in reader.lines().enumerate() {
                                    let line = may_be_line.unwrap();
                                    let existing_line =
                                        lines_to_modify.iter().find(|(st_line, end_line, _)| {
                                            st_line.line == line_num || end_line.line == line_num
                                        });

                                    if let Some(l) = existing_line {
                                        new_lines.push(l.2.to_string());
                                        continue;
                                    }
                                    new_lines.push(line)
                                }
                                let file =
                                    OpenOptions::new().write(true).truncate(true).open(&path);

                                match file {
                                    Ok(mut f) => {
                                        let e = f.write(new_lines.join("\n").as_bytes()).err();
                                        if let Some(x) = e {
                                            println!("{}", x);
                                        } else {
                                            self.modified += 1;
                                        }
                                    }
                                    Err(e) => {
                                        println!("{:#?}", e);
                                    }
                                }
                            }
                            Err(_) => todo!(),
                        }
                    }
                }
                Err(e) => {
                    panic!("{}", e)
                }
            }
        }
    }

    pub fn display_stats(self) {
        println!("Total {} files", self.total);
        println!("Modified {} files", self.modified);
        println!("Ingored {} files", self.ignored);
    }
}

#[cfg(test)]
mod tests {

    use std::path::Path;

    use swc_common::SourceMap;

    use swc_common::errors::{ColorConfig, Handler};
    use swc_common::sync::Lrc;

    use swc_ecma_ast::Module;
    use swc_ecma_parser::TsConfig;
    use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};

    fn _load_test_file(path: &Path) -> Module {
        let cm: Lrc<SourceMap> = Default::default();
        let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));
        let fm = cm.load_file(path).expect("failed to load ");
        let config = TsConfig {
            tsx: true,
            disallow_ambiguous_jsx_like: false,
            ..Default::default()
        };
        let lexer = Lexer::new(
            Syntax::Typescript(config),
            // EsVersion defaults to es5
            Default::default(),
            StringInput::from(&*fm),
            None,
        );

        let mut parser = Parser::new_from(lexer);

        for e in parser.take_errors() {
            e.into_diagnostic(&handler).emit();
        }

        let mut _module = parser.parse_module().expect("failed to parser module");
        return _module;
    }
}
