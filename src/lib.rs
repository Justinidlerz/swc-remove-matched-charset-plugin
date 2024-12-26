use std::borrow::Cow;
use std::default::Default;
use swc_core::ecma::{
    ast::Program,
    ast::{Str},
    transforms::testing::test_inline,
    visit::{as_folder, VisitMut, VisitMutWith},
};
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};
use regex::Regex;
use serde::Deserialize;
use serde_json::from_str;
use swc_core::atoms::Atom;
use swc_core::ecma::ast::{CallExpr, ExportAll, ExportDefaultDecl, Import, ImportDecl, ImportSpecifier, JSXText, NamedExport, TplElement};
use swc_ecma_parser::{EsSyntax, Syntax};

#[derive(Deserialize, Default)]
struct Config {
    replace_with: Option<String>,
    matches: Vec<String>,
}

struct RemoveInvalidContent {
    matchers: Vec<Regex>,
    replace_with: String,
}

impl RemoveInvalidContent {
    fn new(config: Config) -> RemoveInvalidContent {
        Self {
            matchers: config.matches.iter().map(|x| Regex::new(x.as_str()).unwrap()).collect(),
            replace_with: config.replace_with.unwrap_or("".to_string()),
        }
    }

    fn replace_with<'h>(&self, matcher: &Regex, str: &'h str) -> Result<Cow<'h, str>, bool> {
        if !matcher.is_match(str) {
            return Err(false);
        }

        Ok(matcher.replace_all(str, |caps: &regex::Captures| {
            if self.replace_with.is_empty() {
                return "".to_string();
            }

            let matched_str = &caps[0];
            self.replace_with.repeat(matched_str.len())
        }))
    }
}


impl VisitMut for RemoveInvalidContent {
    fn visit_mut_export_all(&mut self, _: &mut ExportAll) {}
    fn visit_mut_named_export(&mut self, _: &mut NamedExport) {}
    fn visit_mut_export_default_decl(&mut self, _: &mut ExportDefaultDecl) {}
    fn visit_mut_import(&mut self, _: &mut Import) {}
    fn visit_mut_import_decl(&mut self, _: &mut ImportDecl) {}
    fn visit_mut_import_specifier(&mut self, _: &mut ImportSpecifier) {}

    fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
        if !node.callee.is_import() {
            node.visit_mut_children_with(self);
        }
    }

    fn visit_mut_jsx_text(&mut self, node: &mut JSXText) {
        for matcher in self.matchers.iter() {
            if let Ok(new_value) = self.replace_with(matcher, node.raw.as_str()) {
                let new_atom = Atom::from(new_value);
                node.raw.clone_from(&new_atom);
                node.value.clone_from(&new_atom);
            }
        }

        node.visit_mut_children_with(self);
    }

    fn visit_mut_str(&mut self, node: &mut Str) {
        for matcher in self.matchers.iter() {
            if let Ok(new_value) = self.replace_with(matcher, node.value.as_str()) {
                node.clone_from(&Str::from(new_value.to_string()))
            }
        }
        node.visit_mut_children_with(self);
    }

    fn visit_mut_tpl_element(&mut self, node: &mut TplElement) {
        for matcher in self.matchers.iter() {
            if let Ok(raw_value) = self.replace_with(matcher, node.raw.as_str()) {
                let cooked_value = Str::from_tpl_raw(&raw_value);
                let new_atom = Atom::from(raw_value);

                let tpl_element = TplElement {
                    span: node.span,
                    tail: node.tail,
                    raw: new_atom,
                    cooked: Some(cooked_value),
                };

                node.clone_from(&tpl_element)
            }
        }
        node.visit_mut_children_with(self);
    }
}


#[plugin_transform]
pub fn process_transform(mut program: Program, data: TransformPluginProgramMetadata) -> Program {
    let config = from_str::<Option<Config>>(
        &data
            .get_transform_plugin_config()
            .expect("failed to get plugin config for swc-remove-matched-charset-plugin"),
    )
        .expect("invalid packages")
        .unwrap_or(Config::default());

    program.visit_mut_with(&mut RemoveInvalidContent::new(config));

    program
}


test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(Config{
        matches: vec![r"[\u4E00-\u9FFF]".to_string()],
        ..Default::default()
    })),
    should_not_change,
    r#"console.log("transform");"#,
    r#"console.log("transform");"#
);

test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(Config{
        matches: vec![r"[\u4E00-\u9FFF]".to_string()],
        ..Default::default()
    })),
    should_remove_in_method_calls,
    r#"console.log("transform中文");"#,
    r#"console.log("transform");"#
);

test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(
        Config{
        matches: vec![r"[\u4E00-\u9FFF]".to_string()],
        ..Default::default()
    }
    )),
    should_remove_in_object_property,
    r#"const a = {
      cde: {
        code: 1,
        message: "视频下载错误",
        description:
          "只要视频下载错误就使用此类型",
      }
     }"#,
    r#"const a = {
      cde: {
        code: 1,
        message: "",
        description: "",
      }
     }"#
);

test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(
        Config{
        matches: vec![r"[\u4E00-\u9FFF]".to_string()],
        ..Default::default()
    }
    )),
    should_left_english_and_special_characters,
    r#"const a = {
      abc: {
        code: 1,
        message: "视频下载错误",
        description:
          "只要视频下载xhr错误就使用，此类型",
      }
     }"#,
    r#"const a = {
      abc: {
        code: 1,
        message: "",
        description: "xhr，",
      }
     }"#
);

test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(Config{
        matches: vec![r"abc.com|cde.org".to_string()],
        ..Default::default()
    })),
    should_remove_url,
    r#"console.log("https://abc.com/faker-url");"#,
    r#"console.log("https:///faker-url");"#
);

test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(
        Config{
        matches: vec![r"[\u4E00-\u9FFF]".to_string()],
        ..Default::default()
    }
    )),
    should_not_remove_slack,
    r#"const a = {
      cde: {
        code: 1,
        message: "\\\\",
      }
     }"#,
    r#"const a = {
      cde: {
        code: 1,
        message: "\\\\",
      }
     }"#
);


test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(
        Config{
        matches: vec![r"[\u4E00-\u9FFF]".to_string()],
        ..Default::default()
    }
    )),
    should_not_remove_slack_from_tpl,
    r#"const a = `\\中文${b}`"#,
    r#"const a = `\\${b}`"#
);

test_inline!(
    Syntax::Es(EsSyntax {
        jsx: true,
        ..Default::default()
    }),
    |_| as_folder(RemoveInvalidContent::new(
        Config{
        matches: vec![r"[\u4E00-\u9FFF]".to_string()],
        ..Default::default()
    }
    )),
    should_remove_chinese_on_jsx,
    r#"const a = () => {
        return <div>关闭
            <p>打开</p>
        </div>
    }
    "#,
    r#"const a = () => {
        return <div>
            <p></p>
        </div>
    }"#
);


test_inline!(
    Syntax::Es(EsSyntax {
        jsx: true,
        ..Default::default()
    }),
    |_| as_folder(RemoveInvalidContent::new(
        Config{
        matches: vec![r"[\u4E00-\u9FFF]".to_string()],
        ..Default::default()
    }
    )),
    should_remove_chinese_on_jsx_attr,
    r#"const a = () => {
        return <div data-info="中文">
            <p>node</p>
        </div>
    }
    "#,
    r#"const a = () => {
        return <div data-info="">
            <p>node</p>
        </div>
    }"#
);


test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(Config{
        matches: vec![r"abc.com|cde.org".to_string()],
        ..Default::default()
    })),
    should_remove_from_tpl,
    r#"console.log(`https://abc.com/faker-url/${window.location.href}`);"#,
    r#"console.log(`https:///faker-url/${window.location.href}`);"#
);

test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(Config{
        matches: vec![r"abc.com|cde.org".to_string()],
        ..Default::default()
    })),
    should_not_remove_from_import_all,
    r#"import * as A from "/abc.com/faker-url";"#,
    r#"import * as A from "/abc.com/faker-url";"#
);

test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(Config{
        matches: vec![r"abc.com|cde.org".to_string()],
        ..Default::default()
    })),
    should_not_remove_from_import_only,
    r#"import "/abc.com/faker-url";"#,
    r#"import "/abc.com/faker-url";"#
);

test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(Config{
        matches: vec![r"abc.com|cde.org".to_string()],
        ..Default::default()
    })),
    should_not_remove_from_default_import,
    r#"import abc from "/abc.com/faker-url";"#,
    r#"import abc from "/abc.com/faker-url";"#
);


test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(Config{
        matches: vec![r"abc.com|cde.org".to_string()],
        ..Default::default()
    })),
    should_not_remove_from_named_import,
    r#"import { efg } from "/abc.com/faker-url";"#,
    r#"import { efg } from "/abc.com/faker-url";"#
);

test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(Config{
        matches: vec![r"abc.com|cde.org".to_string()],
        ..Default::default()
    })),
    should_not_remove_from_dynamic_import,
    r#"import("/abc.com/faker-url");"#,
    r#"import("/abc.com/faker-url");"#
);

test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(Config{
        matches: vec![r"abc.com|cde.org".to_string()],
        ..Default::default()
    })),
    should_not_remove_from_all_export,
    r#"export * from "/abc.com/faker-url";"#,
    r#"export * from "/abc.com/faker-url";"#
);

test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(Config{
        matches: vec![r"abc.com|cde.org".to_string()],
        ..Default::default()
    })),
    should_not_remove_from_all_export_with_rename,
    r#"export * as a from "/abc.com/faker-url";"#,
    r#"export * as a from "/abc.com/faker-url";"#
);

test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(Config{
        matches: vec![r"abc.com|cde.org".to_string()],
        ..Default::default()
    })),
    should_not_remove_from_named_export,
    r#"export { cde } from "/abc.com/faker-url";"#,
    r#"export { cde } from "/abc.com/faker-url";"#
);

test_inline!(
    Default::default(),
    |_| as_folder(RemoveInvalidContent::new(Config{
        matches: vec![r"abc.com|cde.org".to_string()],
        replace_with: Some(String::from("*"))
    })),
    should_replace_as_passed_char,
    r#"console.log("https://abc.com/faker-url");"#,
    r#"console.log("https://*******/faker-url");"#
);


