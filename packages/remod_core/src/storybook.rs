use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

use crate::{
    display_name::{FunctionDelarationWalker, VariableDeclarationWalker},
    utils::{get_program, parse_module, should_ignore_entry},
};
use glob::Paths;
use remod_config::Config;
use swc_common::Spanned;
use swc_common::{chain, util::take::Take, Loc};
use swc_ecma_ast::{BindingIdent, Ident, Program};
use swc_ecma_visit::{Visit, VisitWith};

pub struct StoryNameExpr {
    pub expr: Ident,
    /// Stories to filter from
    _filter: String,
}

impl Default for StoryNameExpr {
    fn default() -> Self {
        Self {
            expr: Ident::dummy(),
            _filter: Default::default(),
        }
    }
}

impl Visit for StoryNameExpr {
    fn visit_binding_ident(&mut self, n: &BindingIdent) {
        if &*n.id.sym == &self._filter {
            self.expr = n.id.clone();
        }
    }
}

#[derive(Debug, Default)]
struct Story {
    /// Name of the story
    name: String,
    /// Type annotation
    type_annotation: String,
    /// Final result structure of the story
    value: String,
}

impl Story {
    fn new(name: String) -> Self {
        Story {
            name,
            type_annotation: String::from("Story"),
            value: Default::default(),
        }
    }
    fn print_story(&mut self) {
        self.value = format!(
            "export const {}_{}: {} = {{\n
                render: (args)=><{} {{...args}} />
            \n}}
            ",
            self.name, "Primary", self.type_annotation, self.name
        );
    }
}

pub struct StoryFile {
    /// component in Process
    component: String,
    /// The default import statement
    import_default: String,
    /// The component import statement
    import_component: String,
    /// Meta declaration
    meta_decl: String,
    /// Story type declaration
    story_type: String,
    /// List of stories
    stories: Vec<Story>,
}

impl Default for StoryFile {
    fn default() -> Self {
        Self {
            component: Default::default(),
            import_default: Default::default(),
            import_component: Default::default(),
            meta_decl: Default::default(),
            story_type: Default::default(),
            stories: Default::default(),
        }
    }
}
impl StoryFile {
    fn new(comp: String, stories: Vec<Story>) -> Self {
        StoryFile {
            component: comp,
            stories,
            ..Default::default()
        }
    }
    fn print_import_default(&mut self) {
        self.import_default = format!("import type {{ Meta, StoryObj }} from '@storybook/react';");
    }
    fn print_import_component(&mut self, file_name: &str) {
        self.import_component = format!("import {{ {} }} from \'./{}\'", self.component, file_name);
    }
    fn print_meta_decl(&mut self) {
        self.meta_decl = format!(
            "const meta: Meta<typeof {}> = {{\n
                component: {},
            \n}};\n
            export default meta;
            ",
            self.component, self.component
        );
    }
    fn print_story_type(&mut self) {
        self.story_type = format!("type Story = StoryObj<typeof {}>;", self.component);
    }

    pub fn emit_story_file(&self) -> String {
        format!(
            "{}\n{}\n{}\n{}\n{}",
            self.import_default,
            self.import_component,
            self.meta_decl,
            self.story_type,
            self.stories
                .iter()
                .map(|s| s.value.to_owned())
                .collect::<Vec<String>>()
                .join("\n")
        )
    }

    pub fn find_story_ident_loc(&self, source_file: &PathBuf, config: &Config) -> (Loc, Loc) {
        let (program, cm, _comments) = get_program(source_file, config);
        let mut story_expr = StoryNameExpr {
            _filter: format!("{}_Primary", self.component.to_owned()),
            ..Default::default()
        };
        program.visit_with(&mut story_expr);
        let start_loc = cm.lookup_char_pos(story_expr.expr.span_lo());
        let end_loc = cm.lookup_char_pos(story_expr.expr.span_hi());
        (start_loc, end_loc)
    }
}

#[derive(Debug, Default)]
pub struct Storybook {
    ///  Count of modified files
    pub created: i64,
    /// Count of ignored files
    pub ignored: i64,
    /// Count of total files
    pub total: i64,
}

impl Storybook {
    pub fn emit_story_file(&mut self, path: &PathBuf, config: &Config) {
        if should_ignore_entry(&config.ignore, path) {
            self.ignored += 1;
        } else {
            let (module, _cm, _comments) = parse_module(path, config);
            let mut variable_decl_visitor = VariableDeclarationWalker { variables: vec![] };
            let mut function_decl_visitor = FunctionDelarationWalker {
                function_decls: vec![],
            };
            let mut visitor = chain!(&mut variable_decl_visitor, &mut function_decl_visitor);
            let program = Program::Module(module);
            program.visit_with(&mut visitor);
            let mut story_name: Option<String> = Some(String::from(""));
            if variable_decl_visitor.variables.len() > 0 {
                let variable = variable_decl_visitor.variables.get(0);
                if let Some(v) = variable {
                    story_name = Some(v.sym.to_string());
                }
            } else if function_decl_visitor.function_decls.len() > 0 {
                let variable = function_decl_visitor.function_decls.get(0);
                if let Some(v) = variable {
                    story_name = Some(v.sym.to_string());
                }
            }
            if let Some(component) = story_name {
                let mut stories: Vec<Story> = vec![];
                let mut story = Story::new(component.clone());
                story.print_story();
                stories.push(story);
                let mut story_file = StoryFile::new(component.to_owned(), stories);
                let file_name = path.file_stem().unwrap().to_str().unwrap();
                story_file.print_import_default();
                story_file.print_import_component(file_name);
                story_file.print_meta_decl();
                story_file.print_story_type();
                let final_output = story_file.emit_story_file();
                let may_be_dir = path.parent();
                match may_be_dir {
                    Some(dir_path) => {
                        let directory = dir_path.display();
                        let final_path = format!("{}/{}", directory, file_name);
                        let pattern_matches = vec![
                            String::from(final_path.to_owned() + ".stories.tsx"),
                            String::from(final_path.to_owned() + ".story.tsx"),
                        ];
                        if pattern_matches.iter().any(|p| {
                            let exists_already = File::open(p);
                            match exists_already {
                                Ok(_) => true,
                                Err(_) => false,
                            }
                        }) {
                            println!("Story already exists for {}", path.display());
                            self.ignored += 0;
                        } else {
                            let ext = config
                                .story_file_ext
                                .to_owned()
                                .unwrap_or(String::from(".stories.tsx"));
                            let file_name = format!("{}{}", final_path, ext);
                            let new_path = Path::new(file_name.as_str());
                            let exists_already = File::open(new_path);
                            match exists_already {
                                Ok(_) => {
                                    println!("Story alredy exists for {}", path.display());
                                    self.ignored += 1;
                                }
                                Err(_) => {
                                    let may_be_file =
                                        OpenOptions::new().write(true).create(true).open(new_path);
                                    match may_be_file {
                                        Ok(mut file) => {
                                            println!(
                                                "{} => {}",
                                                path.display(),
                                                new_path.display()
                                            );
                                            let _ = file.write(final_output.as_bytes());
                                            self.created += 1;
                                        }
                                        Err(e) => {
                                            println!("{:#?}", e);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    None => {
                        println!("Not a recognisable directory");
                    }
                }
            }
        }
    }

    pub fn pre_process_story_module(
        &mut self,
        story_name: Option<String>,
        path: &PathBuf,
        config: &Config,
    ) -> Result<(StoryFile, String), String> {
        if let Some(component) = story_name {
            let mut stories: Vec<Story> = vec![];
            let mut story = Story::new(component.clone());
            story.print_story();
            stories.push(story);
            let mut story_file = StoryFile::new(component.to_owned(), stories);
            let file_name = path.file_stem().unwrap().to_str().unwrap();
            story_file.print_import_default();
            story_file.print_import_component(file_name);
            story_file.print_meta_decl();
            story_file.print_story_type();
            let may_be_dir = path.parent();
            let result = match may_be_dir {
                Some(dir_path) => {
                    let directory = dir_path.display();
                    let final_path = format!("{}/{}", directory, file_name);
                    let pattern_matches = vec![
                        String::from(final_path.to_owned() + ".stories.tsx"),
                        String::from(final_path.to_owned() + ".story.tsx"),
                    ];
                    if pattern_matches.iter().any(|p| {
                        let exists_already = File::open(p);
                        match exists_already {
                            Ok(_) => true,
                            Err(_) => false,
                        }
                    }) {
                        Err(format!("Story already exists for {}", path.display()))
                    } else {
                        let ext = config
                            .story_file_ext
                            .to_owned()
                            .unwrap_or(String::from(".stories.tsx"));
                        let file_name = format!("{}{}", final_path, ext);
                        let new_path = Path::new(&file_name);
                        let exists_already = File::open(new_path);
                        match exists_already {
                            Ok(_) => Err(format!("Story already exists for {}", path.display())),
                            Err(_) => Ok((story_file, file_name)),
                        }
                    }
                }
                None => Err("Not a recognisable directory".to_string()),
            };
            return result;
        }
        Err("Could not complete request".to_string())
    }

    pub fn emit_story_files(&mut self, files: Paths, config: &Config) {
        for entry in files {
            self.total += 1;
            match entry {
                Ok(path) => self.emit_story_file(&path, config),
                Err(_) => {}
            }
        }
    }

    pub fn display_stats(self) {
        println!("Total {} files", self.total);
        println!("Modified {} files", self.created);
        println!("Ingored {} files", self.ignored);
    }
}

#[cfg(test)]
mod test {
    use glob::glob;

    use super::*;

    #[test]
    fn test_storybook() {
        let files = glob("tests/arrow_functions/normal_expression.tsx").unwrap();
        let mut storybook = Storybook {
            ..Default::default()
        };
        let config = Config {
            typescript: Some(true),
            ..Config::default()
        };
        for entry in files {
            match entry {
                Ok(path) => storybook.emit_story_file(&path, &config),
                Err(_) => todo!(),
            }
        }
    }
}
