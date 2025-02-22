mod array_builder;
mod forward_decl;
mod includes;
mod method;

use std::collections::BTreeSet;

use camino::{Utf8Path, Utf8PathBuf};
use itertools::Itertools;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use rayon::prelude::*;

use crate::codegen::common::write_file;
use crate::{
    codegen::autogen_warning, ArrowRegistry, Docs, ElementType, ObjectField, ObjectKind, Objects,
    Type,
};
use crate::{Object, ObjectSpecifics};

use self::array_builder::{
    arrow_array_builder_type, arrow_array_builder_type_object,
    quote_arrow_array_builder_type_instantiation,
};
use self::forward_decl::{ForwardDecl, ForwardDecls};
use self::includes::Includes;
use self::method::{Method, MethodDeclaration};

// Special strings we insert as tokens, then search-and-replace later.
// This is so that we can insert comments and whitespace into the generated code.
// `TokenStream` ignores whitespace (including comments), but we can insert "quoted strings",
// so that is what we do.
const NEWLINE_TOKEN: &str = "NEWLINE_TOKEN";
const NORMAL_COMMENT_PREFIX_TOKEN: &str = "NORMAL_COMMENT_PREFIX_TOKEN";
const NORMAL_COMMENT_SUFFIX_TOKEN: &str = "NORMAL_COMMENT_SUFFIX_TOKEN";
const DOC_COMMENT_PREFIX_TOKEN: &str = "DOC_COMMENT_PREFIX_TOKEN";
const DOC_COMMENT_SUFFIX_TOKEN: &str = "DOC_COMMENT_SUFFIX_TOKEN";
const ANGLE_BRACKET_LEFT_TOKEN: &str = "SYS_INCLUDE_PATH_PREFIX_TOKEN";
const ANGLE_BRACKET_RIGHT_TOKEN: &str = "SYS_INCLUDE_PATH_SUFFIX_TOKEN";
const HEADER_EXTENSION_PREFIX_TOKEN: &str = "HEADER_EXTENSION_PREFIX_TOKEN";
const HEADER_EXTENSION_SUFFIX_TOKEN: &str = "HEADER_EXTENSION_SUFFIX_TOKEN";
const TODO_TOKEN: &str = "TODO_TOKEN";

fn quote_comment(text: &str) -> TokenStream {
    quote! { #NORMAL_COMMENT_PREFIX_TOKEN #text #NORMAL_COMMENT_SUFFIX_TOKEN }
}

fn quote_doc_comment(text: &str) -> TokenStream {
    quote! { #DOC_COMMENT_PREFIX_TOKEN #text #DOC_COMMENT_SUFFIX_TOKEN }
}

fn string_from_token_stream(token_stream: &TokenStream, source_path: Option<&Utf8Path>) -> String {
    let mut code = String::new();
    code.push_str(&format!("// {}\n", autogen_warning!()));
    if let Some(source_path) = source_path {
        code.push_str(&format!("// Based on {source_path:?}.\n"));
    }

    code.push('\n');

    let generated_code = token_stream
        .to_string()
        .replace(&format!("{NEWLINE_TOKEN:?}"), "\n")
        .replace(NEWLINE_TOKEN, "\n") // Should only happen inside header extensions.
        .replace(&format!("{NORMAL_COMMENT_PREFIX_TOKEN:?} \""), "//")
        .replace(&format!("\" {NORMAL_COMMENT_SUFFIX_TOKEN:?}"), "\n")
        .replace(&format!("{DOC_COMMENT_PREFIX_TOKEN:?} \""), "///")
        .replace(&format!("\" {DOC_COMMENT_SUFFIX_TOKEN:?}"), "\n")
        .replace(&format!("{HEADER_EXTENSION_PREFIX_TOKEN:?} \""), "")
        .replace(&format!("\" {HEADER_EXTENSION_SUFFIX_TOKEN:?}"), "")
        .replace(&format!("{ANGLE_BRACKET_LEFT_TOKEN:?} \""), "<")
        .replace(&format!("\" {ANGLE_BRACKET_RIGHT_TOKEN:?}"), ">")
        .replace(
            &format!("{TODO_TOKEN:?}"),
            "\n// TODO(#2647): code-gen for C++\n",
        )
        .replace("< ", "<")
        .replace(" >", ">")
        .replace(" ::", "::");

    // Need to fix escaped quotes inside of comments.
    // Walk through all comments, replace and push to `code` as we go.
    let mut last_comment_end = 0;
    while let Some(comment_start) = generated_code[last_comment_end..].find("//") {
        code.push_str(&generated_code[last_comment_end..last_comment_end + comment_start]);
        let comment_start = last_comment_end + comment_start;
        let comment_end = comment_start + generated_code[comment_start..].find('\n').unwrap();
        let comment = &generated_code[comment_start..comment_end];
        let comment = comment.replace("\\\"", "\"");
        let comment = comment.replace("\\\\", "\\");
        code.push_str(&comment);
        last_comment_end = comment_end;
    }
    code.push_str(&generated_code[last_comment_end..]);

    code.push('\n');

    code = clang_format::clang_format_with_style(&code, &clang_format::ClangFormatStyle::File)
        .expect("Failed to run clang-format");

    code
}

pub struct CppCodeGenerator {
    output_path: Utf8PathBuf,
}

impl CppCodeGenerator {
    pub fn new(output_path: impl Into<Utf8PathBuf>) -> Self {
        Self {
            output_path: output_path.into(),
        }
    }

    fn generate_folder(
        &self,
        objects: &Objects,
        object_kind: ObjectKind,
        folder_name: &str,
    ) -> BTreeSet<Utf8PathBuf> {
        let folder_path_sdk = self.output_path.join("src/rerun").join(folder_name);
        let folder_path_testing = self.output_path.join("tests/generated").join(folder_name);
        let mut filepaths = BTreeSet::default();

        // Generate folder contents:
        let ordered_objects = objects.ordered_objects(object_kind.into());
        for &obj in &ordered_objects {
            let filename = obj.snake_case_name();

            let mut hpp_includes = Includes::new(obj.fqname.clone());
            hpp_includes.insert_system("cstdint"); // we use `uint32_t` etc everywhere.
            hpp_includes.insert_rerun("result.hpp"); // rerun result is used for serialization methods

            let hpp_type_extensions =
                hpp_type_extensions(&folder_path_sdk, &filename, &mut hpp_includes);

            let (hpp, cpp) = generate_hpp_cpp(objects, obj, hpp_includes, &hpp_type_extensions);

            for (extension, tokens) in [("hpp", hpp), ("cpp", cpp)] {
                let string = string_from_token_stream(&tokens, obj.relative_filepath());
                let folder_path = if obj.is_testing() {
                    &folder_path_testing
                } else {
                    &folder_path_sdk
                };
                let filepath = folder_path.join(format!("{filename}.{extension}"));
                write_file(&filepath, string);
                let inserted = filepaths.insert(filepath);
                assert!(
                    inserted,
                    "Multiple objects with the same name: {:?}",
                    obj.name
                );
            }
        }

        // Generate module file that includes all the headers:
        for testing in [false, true] {
            let hash = quote! { # };
            let pragma_once = pragma_once();
            let header_file_names = ordered_objects
                .iter()
                .filter(|obj| obj.is_testing() == testing)
                .map(|obj| format!("{folder_name}/{}.hpp", obj.snake_case_name()));
            let tokens = quote! {
                #pragma_once
                #(#hash include #header_file_names "NEWLINE_TOKEN")*
            };
            let folder_path = if testing {
                &folder_path_testing
            } else {
                &folder_path_sdk
            };
            let filepath = folder_path
                .parent()
                .unwrap()
                .join(format!("{folder_name}.hpp"));
            let string = string_from_token_stream(&tokens, None);
            write_file(&filepath, string);
            filepaths.insert(filepath);
        }

        super::common::remove_old_files_from_folder(folder_path_sdk, &filepaths);

        filepaths
    }
}

impl crate::CodeGenerator for CppCodeGenerator {
    fn generate(
        &mut self,
        objects: &Objects,
        _arrow_registry: &ArrowRegistry,
    ) -> BTreeSet<Utf8PathBuf> {
        ObjectKind::ALL
            .par_iter()
            .map(|object_kind| {
                let folder_name = object_kind.plural_snake_case();
                self.generate_folder(objects, *object_kind, folder_name)
            })
            .flatten()
            .collect()
    }
}

/// Retrieves code from an extension cpp file that should go to the generated header.
///
/// Additionally, picks up all includes files that aren't including the header itself.
fn hpp_type_extensions(
    folder_path: &Utf8Path,
    filename: &str,
    includes: &mut Includes,
) -> TokenStream {
    let extension_file = folder_path.join(format!("{filename}_ext.cpp"));
    let Ok(content) = std::fs::read_to_string(extension_file.as_std_path()) else {
        return quote! {};
    };

    let target_header = format!("{filename}.hpp");
    for line in content.lines() {
        if line.starts_with("#include") {
            if let Some(start) = line.find('\"') {
                let end = line.rfind('\"').unwrap_or_else(|| {
                    panic!("Expected to find '\"' include line {line} in file {extension_file:?}")
                });

                let include = &line[start + 1..end];
                if include != target_header {
                    includes.insert_relative(include);
                }
            } else if let Some(start) = line.find('<') {
                let end = line.rfind('>').unwrap_or_else(|| {
                    panic!(
                        "Expected to find or '>' in include line {line} in file {extension_file:?}"
                    )
                });
                includes.insert_system(&line[start + 1..end]);
            } else {
                panic!("Expected to find '\"' or '<' in include line {line} in file {extension_file:?}");
            }
        }
    }

    const COPY_TO_HEADER_START_MARKER: &str = "[CODEGEN COPY TO HEADER START]";
    const COPY_TO_HEADER_END_MARKER: &str = "[CODEGEN COPY TO HEADER END]";

    let start = content.find(COPY_TO_HEADER_START_MARKER).unwrap_or_else(||
        panic!("C++ extension file missing start marker. Without it, nothing is exposed to the header, i.e. not accessible to the user. Expected to find '{COPY_TO_HEADER_START_MARKER}' in {extension_file:?}")
    );

    let end = content.find(COPY_TO_HEADER_END_MARKER).unwrap_or_else(||
        panic!("C++ extension file has a start marker but no end marker. Expected to find '{COPY_TO_HEADER_START_MARKER}' in {extension_file:?}")
    );
    let end = content[..end].rfind('\n').unwrap_or_else(||
        panic!("Expected line break at some point before {COPY_TO_HEADER_END_MARKER} in {extension_file:?}")
    );

    let comment = quote_comment(&format!(
        "Extensions to generated type defined in '{}'",
        extension_file.file_name().unwrap()
    ));
    let extensions = &content[start + COPY_TO_HEADER_START_MARKER.len()..end]
        .replace('\n', &format!(" {NEWLINE_TOKEN} "));
    quote! {
        public:
        #NEWLINE_TOKEN
        #comment
        #NEWLINE_TOKEN
        #HEADER_EXTENSION_PREFIX_TOKEN #extensions #HEADER_EXTENSION_SUFFIX_TOKEN
        #NEWLINE_TOKEN
    }
}

fn generate_hpp_cpp(
    objects: &Objects,
    obj: &Object,
    hpp_includes: Includes,
    hpp_type_extensions: &TokenStream,
) -> (TokenStream, TokenStream) {
    let QuotedObject { hpp, cpp } =
        QuotedObject::new(objects, obj, hpp_includes, hpp_type_extensions);
    let snake_case_name = obj.snake_case_name();
    let hash = quote! { # };
    let pragma_once = pragma_once();
    let header_file_name = format!("{snake_case_name}.hpp");

    let hpp = quote! {
        #pragma_once
        #hpp
    };
    let cpp = quote! {
        #hash include #header_file_name #NEWLINE_TOKEN #NEWLINE_TOKEN
        #cpp
    };

    (hpp, cpp)
}

fn pragma_once() -> TokenStream {
    let hash = quote! { # };
    quote! {
        #hash pragma once #NEWLINE_TOKEN #NEWLINE_TOKEN
    }
}

struct QuotedObject {
    hpp: TokenStream,
    cpp: TokenStream,
}

impl QuotedObject {
    pub fn new(
        objects: &Objects,
        obj: &Object,
        hpp_includes: Includes,
        hpp_type_extensions: &TokenStream,
    ) -> Self {
        match obj.specifics {
            crate::ObjectSpecifics::Struct => {
                Self::from_struct(objects, obj, hpp_includes, hpp_type_extensions)
            }
            crate::ObjectSpecifics::Union { .. } => {
                Self::from_union(objects, obj, hpp_includes, hpp_type_extensions)
            }
        }
    }

    fn from_struct(
        objects: &Objects,
        obj: &Object,
        mut hpp_includes: Includes,
        hpp_type_extensions: &TokenStream,
    ) -> QuotedObject {
        let namespace_ident = format_ident!("{}", obj.kind.plural_snake_case()); // `datatypes`, `components`, or `archetypes`
        let type_name = &obj.name;
        let type_ident = format_ident!("{type_name}"); // The PascalCase name of the object type.
        let quoted_docs = quote_docstrings(&obj.docs);

        let mut cpp_includes = Includes::new(obj.fqname.clone());
        #[allow(unused)]
        let mut hpp_declarations = ForwardDecls::default();

        let field_declarations = obj
            .fields
            .iter()
            .map(|obj_field| {
                let declaration = quote_variable_with_docstring(
                    &mut hpp_includes,
                    obj_field,
                    &format_ident!("{}", obj_field.name),
                );
                quote! {
                    #NEWLINE_TOKEN
                    #declaration
                }
            })
            .collect_vec();

        let (constants_hpp, constants_cpp) =
            quote_constants_header_and_cpp(obj, objects, &type_ident);
        let mut methods = Vec::new();

        match obj.kind {
            ObjectKind::Datatype | ObjectKind::Component => {
                if obj.fields.len() == 1 {
                    methods.extend(single_field_constructor_methods(
                        obj,
                        &mut hpp_includes,
                        &type_ident,
                        objects,
                    ));
                };

                // Arrow serialization methods.
                // TODO(andreas): These are just utilities for to_data_cell. How do we hide them best from the public header?
                methods.push(arrow_data_type_method(
                    obj,
                    objects,
                    &mut hpp_includes,
                    &mut cpp_includes,
                    &mut hpp_declarations,
                ));
                methods.push(new_arrow_array_builder_method(
                    obj,
                    objects,
                    &mut hpp_includes,
                    &mut cpp_includes,
                    &mut hpp_declarations,
                ));
                methods.push(fill_arrow_array_builder_method(
                    obj,
                    &type_ident,
                    &mut cpp_includes,
                    &mut hpp_declarations,
                    objects,
                ));

                if obj.kind == ObjectKind::Component {
                    methods.push(component_to_data_cell_method(
                        &type_ident,
                        &mut hpp_includes,
                        &mut cpp_includes,
                    ));
                }
            }
            ObjectKind::Archetype => {
                hpp_includes.insert_system("utility"); // std::move

                let required_component_fields = obj
                    .fields
                    .iter()
                    .filter(|field| !field.is_nullable)
                    .collect_vec();

                // Constructors with all required components.
                {
                    let (arguments, assignments): (Vec<_>, Vec<_>) = required_component_fields
                        .iter()
                        .map(|obj_field| {
                            let field_ident = format_ident!("{}", obj_field.name);
                            let arg_ident = format_ident!("_{}", obj_field.name);
                            (
                                quote_variable(&mut hpp_includes, obj_field, &arg_ident),
                                quote! { #field_ident(std::move(#arg_ident)) },
                            )
                        })
                        .unzip();

                    methods.push(Method {
                        declaration: MethodDeclaration::constructor(quote! {
                            #type_ident(#(#arguments),*) : #(#assignments),*
                        }),
                        ..Method::default()
                    });

                    // Provide a non-array version if there's any vectors.
                    if required_component_fields
                        .iter()
                        .any(|obj_field| matches!(obj_field.typ, Type::Vector { .. }))
                    {
                        let (arguments, assignments): (Vec<_>, Vec<_>) = required_component_fields
                            .iter()
                            .map(|obj_field| {
                                let field_ident = format_ident!("{}", obj_field.name);
                                let arg_ident = format_ident!("_{}", obj_field.name);

                                if let Type::Vector { elem_type } = &obj_field.typ {
                                    let elem_type =
                                        quote_element_type(&mut hpp_includes, elem_type);
                                    (
                                        quote! { #elem_type #arg_ident },
                                        quote! { #field_ident(1, std::move(#arg_ident)) },
                                    )
                                } else {
                                    (
                                        quote_variable(&mut hpp_includes, obj_field, &arg_ident),
                                        quote! { #field_ident(std::move(#arg_ident)) },
                                    )
                                }
                            })
                            .unzip();
                        methods.push(Method {
                            declaration: MethodDeclaration::constructor(quote! {
                                #type_ident(#(#arguments),*) : #(#assignments),*
                            }),
                            ..Method::default()
                        });
                    }
                }
                // Builder methods for all optional components.
                for obj_field in obj.fields.iter().filter(|field| field.is_nullable) {
                    let field_ident = format_ident!("{}", obj_field.name);
                    // C++ compilers give warnings for re-using the same name as the member variable.
                    let parameter_ident = format_ident!("_{}", obj_field.name);
                    let method_ident = format_ident!("with_{}", obj_field.name);
                    let non_nullable = ObjectField {
                        is_nullable: false,
                        ..obj_field.clone()
                    };
                    let parameter_declaration =
                        quote_variable(&mut hpp_includes, &non_nullable, &parameter_ident);
                    methods.push(Method {
                        docs: obj_field.docs.clone().into(),
                        declaration: MethodDeclaration {
                            is_static: false,
                            return_type: quote!(#type_ident&),
                            name_and_parameters: quote! {
                                #method_ident(#parameter_declaration)
                            },
                        },
                        definition_body: quote! {
                            #field_ident = std::move(#parameter_ident);
                            return *this;
                        },
                        inline: true,
                    });

                    // Provide a non-array version if it's a vector.
                    if let Type::Vector { elem_type } = &obj_field.typ {
                        hpp_includes.insert_system("vector"); // std::vector
                        let elem_type = quote_element_type(&mut hpp_includes, elem_type);
                        methods.push(Method {
                            docs: obj_field.docs.clone().into(),
                            declaration: MethodDeclaration {
                                is_static: false,
                                return_type: quote!(#type_ident&),
                                name_and_parameters: quote! {
                                    #method_ident(#elem_type #parameter_ident)
                                },
                            },
                            definition_body: quote! {
                                #field_ident = std::vector(1, std::move(#parameter_ident));
                                return *this;
                            },
                            inline: true,
                        });
                    }
                }

                // Num instances gives the number of primary instances.
                {
                    let first_required_field = required_component_fields.first().unwrap();
                    let first_required_field_name = &format_ident!("{}", first_required_field.name);
                    let definition_body = if first_required_field.typ.is_plural() {
                        quote!(return #first_required_field_name.size();)
                    } else {
                        quote!(return 1;)
                    };
                    methods.push(Method {
                        docs: "Returns the number of primary instances of this archetype.".into(),
                        declaration: MethodDeclaration {
                            is_static: false,
                            return_type: quote!(size_t),
                            name_and_parameters: quote! {
                                num_instances() const
                            },
                        },
                        definition_body,
                        inline: true,
                    });
                }

                methods.push(archetype_to_data_cells(
                    obj,
                    &mut hpp_includes,
                    &mut cpp_includes,
                ));
            }
        };

        let hpp_method_section = if methods.is_empty() {
            quote! {}
        } else {
            let methods_hpp = methods.iter().map(|m| m.to_hpp_tokens());
            quote! {
                public:
                    #type_ident() = default;
                    #NEWLINE_TOKEN
                    #NEWLINE_TOKEN
                    #(#methods_hpp)*
            }
        };
        let hpp = quote! {
            #hpp_includes

            #hpp_declarations

            namespace rerun {
                namespace #namespace_ident {
                    #quoted_docs
                    struct #type_ident {
                        #(#field_declarations;)*

                        #(#constants_hpp;)*

                        #hpp_type_extensions

                        #hpp_method_section
                    };
                }
            }
        };

        let methods_cpp = methods.iter().map(|m| m.to_cpp_tokens(&type_ident));
        let cpp = quote! {
            #cpp_includes

            namespace rerun {
                namespace #namespace_ident {
                    #(#constants_cpp;)*

                    #(#methods_cpp)*
                }
            }
        };

        Self { hpp, cpp }
    }

    fn from_union(
        objects: &Objects,
        obj: &Object,
        mut hpp_includes: Includes,
        hpp_type_extensions: &TokenStream,
    ) -> QuotedObject {
        // We implement sum-types as tagged unions;
        // Putting non-POD types in a union requires C++11.
        //
        // enum class Rotation3DTag : uint8_t {
        //     NONE = 0,
        //     Quaternion,
        //     AxisAngle,
        // };
        //
        // union Rotation3DData {
        //     Quaternion quaternion;
        //     AxisAngle axis_angle;
        // };
        //
        // struct Rotation3D {
        //     Rotation3DTag _tag;
        //     Rotation3DData _data;
        // };

        assert!(
            obj.kind != ObjectKind::Archetype,
            "Union archetypes are not supported {}",
            obj.fqname
        );
        let namespace_ident = format_ident!("{}", obj.kind.plural_snake_case()); // `datatypes` or `components`
        let pascal_case_name = &obj.name;
        let pascal_case_ident = format_ident!("{pascal_case_name}"); // The PascalCase name of the object type.
        let quoted_docs = quote_docstrings(&obj.docs);

        let tag_typename = format_ident!("{pascal_case_name}Tag");
        let data_typename = format_ident!("{pascal_case_name}Data");

        let tag_fields = std::iter::once({
            let comment = quote_doc_comment(
                "Having a special empty state makes it possible to implement move-semantics. \
                We need to be able to leave the object in a state which we can run the destructor on.");
            let tag_name = format_ident!("NONE");
            quote! {
                #NEWLINE_TOKEN
                #comment
                #tag_name = 0,
            }
        })
        .chain(obj.fields.iter().map(|obj_field| {
            let ident = format_ident!("{}", obj_field.name);
            quote! {
                #ident,
            }
        }))
        .collect_vec();

        hpp_includes.insert_system("utility"); // std::move
        hpp_includes.insert_system("cstring"); // std::memcpy

        let mut cpp_includes = Includes::new(obj.fqname.clone());
        #[allow(unused)]
        let mut hpp_declarations = ForwardDecls::default();

        let enum_data_declarations = obj
            .fields
            .iter()
            .map(|obj_field| {
                let declaration = quote_variable_with_docstring(
                    &mut hpp_includes,
                    obj_field,
                    &format_ident!("{}", crate::to_snake_case(&obj_field.name)),
                );
                quote! {
                    #NEWLINE_TOKEN
                    #declaration
                }
            })
            .collect_vec();

        let (constants_hpp, constants_cpp) =
            quote_constants_header_and_cpp(obj, objects, &pascal_case_ident);
        let mut methods = Vec::new();

        // Add one static constructor for every field.
        for obj_field in &obj.fields {
            methods.push(static_constructor_for_enum_type(
                objects,
                &mut hpp_includes,
                obj_field,
                &pascal_case_ident,
                &tag_typename,
            ));
        }

        if are_types_disjoint(&obj.fields) {
            // Implicit construct from the different variant types:
            for obj_field in &obj.fields {
                let snake_case_ident = format_ident!("{}", crate::to_snake_case(&obj_field.name));
                let param_declaration =
                    quote_variable(&mut hpp_includes, obj_field, &snake_case_ident);

                methods.push(Method {
                    docs: obj_field.docs.clone().into(),
                    declaration: MethodDeclaration::constructor(quote!(#pascal_case_ident(#param_declaration))),
                    definition_body: quote!(*this = #pascal_case_ident::#snake_case_ident(std::move(#snake_case_ident));),
                    inline: true,
                });
            }
        } else {
            // Cannot make implicit constructors, e.g. for
            // `enum Angle { Radians(f32), Degrees(f32) };`
        };

        methods.push(arrow_data_type_method(
            obj,
            objects,
            &mut hpp_includes,
            &mut cpp_includes,
            &mut hpp_declarations,
        ));
        methods.push(new_arrow_array_builder_method(
            obj,
            objects,
            &mut hpp_includes,
            &mut cpp_includes,
            &mut hpp_declarations,
        ));
        methods.push(fill_arrow_array_builder_method(
            obj,
            &pascal_case_ident,
            &mut cpp_includes,
            &mut hpp_declarations,
            objects,
        ));

        let destructor = if obj.has_default_destructor(objects) {
            // No destructor needed
            quote! {}
        } else {
            let destructor_match_arms = std::iter::once({
                let comment = quote_comment("Nothing to destroy");
                quote! {
                    case detail::#tag_typename::NONE: {
                        break; #comment
                    }
                }
            })
            .chain(obj.fields.iter().map(|obj_field| {
                let tag_ident = format_ident!("{}", obj_field.name);
                let field_ident = format_ident!("{}", crate::to_snake_case(&obj_field.name));

                if obj_field.typ.has_default_destructor(objects) {
                    let comment = quote_comment("has a trivial destructor");
                    quote! {
                        case detail::#tag_typename::#tag_ident: {
                            break; #comment
                        }
                    }
                } else if let Type::Array { elem_type, length } = &obj_field.typ {
                    // We need special casing for destroying arrays in C++:
                    let elem_type = quote_element_type(&mut hpp_includes, elem_type);
                    let length = proc_macro2::Literal::usize_unsuffixed(*length);
                    quote! {
                        case detail::#tag_typename::#tag_ident: {
                            typedef #elem_type TypeAlias;
                            for (size_t i = #length; i > 0; i -= 1) {
                                _data.#field_ident[i-1].~TypeAlias();
                            }
                            break;
                        }
                    }
                } else {
                    let typedef_declaration =
                        quote_variable(&mut hpp_includes, obj_field, &format_ident!("TypeAlias"));
                    hpp_includes.insert_system("utility"); // std::move
                    quote! {
                        case detail::#tag_typename::#tag_ident: {
                            typedef #typedef_declaration;
                            _data.#field_ident.~TypeAlias();
                            break;
                        }
                    }
                }
            }))
            .collect_vec();

            quote! {
                ~#pascal_case_ident() {
                    switch (this->_tag) {
                        #(#destructor_match_arms)*
                    }
                }
            }
        };

        let copy_constructor = {
            // Note that `switch` on an enum without handling all cases causes `-Wswitch-enum` warning!
            let mut copy_match_arms = Vec::new();
            let mut default_match_arms = Vec::new();
            for obj_field in &obj.fields {
                let tag_ident = format_ident!("{}", obj_field.name);
                let case = quote!(case detail::#tag_typename::#tag_ident:);

                // Inferring from trivial destructability that we don't need to call the copy constructor is a little bit wonky,
                // but is typically the reason why we need to do this in the first place - if we'd always memcpy we'd get double-free errors.
                // (As with swap, we generously assume that objects are rellocatable)
                if obj_field.typ.has_default_destructor(objects) {
                    default_match_arms.push(case);
                } else {
                    let field_ident = format_ident!("{}", crate::to_snake_case(&obj_field.name));
                    copy_match_arms.push(quote! {
                        #case {
                            _data.#field_ident = other._data.#field_ident;
                            break;
                        }
                    });
                }
            }

            let trivial_memcpy = quote! {
                const void* otherbytes = reinterpret_cast<const void*>(&other._data);
                void* thisbytes = reinterpret_cast<void*>(&this->_data);
                std::memcpy(thisbytes, otherbytes, sizeof(detail::#data_typename));
            };

            if copy_match_arms.is_empty() {
                quote!(#pascal_case_ident(const #pascal_case_ident& other) : _tag(other._tag) {
                    #trivial_memcpy
                })
            } else {
                quote!(#pascal_case_ident(const #pascal_case_ident& other) : _tag(other._tag) {
                    switch (other._tag) {
                        #(#copy_match_arms)*

                        case detail::#tag_typename::NONE:
                        #(#default_match_arms)*
                        #trivial_memcpy
                            break;
                    }
                })
            }
        };

        let swap_comment = quote_comment("This bitwise swap would fail for self-referential types, but we don't have any of those.");

        let methods_hpp = methods.iter().map(|m| m.to_hpp_tokens());
        let hpp = quote! {
            #hpp_includes

            #hpp_declarations

            namespace rerun {
                namespace #namespace_ident {
                    namespace detail {
                        enum class #tag_typename : uint8_t {
                            #(#tag_fields)*
                        };

                        union #data_typename {
                            #(#enum_data_declarations;)*

                            #data_typename() { } // Required by static constructors
                            ~#data_typename() { }

                            // Note that this type is *not* copyable unless all enum fields are trivially destructable.

                            void swap(#data_typename& other) noexcept {
                                #NEWLINE_TOKEN
                                #swap_comment
                                char temp[sizeof(#data_typename)];
                                void* otherbytes = reinterpret_cast<void*>(&other);
                                void* thisbytes = reinterpret_cast<void*>(this);
                                std::memcpy(temp, thisbytes, sizeof(#data_typename));
                                std::memcpy(thisbytes, otherbytes, sizeof(#data_typename));
                                std::memcpy(otherbytes, temp, sizeof(#data_typename));
                            }
                        };

                    }

                    #quoted_docs
                    struct #pascal_case_ident {
                        #(#constants_hpp;)*

                        #pascal_case_ident() : _tag(detail::#tag_typename::NONE) {}

                        #copy_constructor

                        // Copy-assignment
                        #pascal_case_ident& operator=(const #pascal_case_ident& other) noexcept {
                            #pascal_case_ident tmp(other);
                            this->swap(tmp);
                            return *this;
                        }

                        // Move-constructor:
                        #pascal_case_ident(#pascal_case_ident&& other) noexcept : _tag(detail::#tag_typename::NONE) {
                            this->swap(other);
                        }

                        // Move-assignment:
                        #pascal_case_ident& operator=(#pascal_case_ident&& other) noexcept {
                            this->swap(other);
                            return *this;
                        }

                        #destructor

                        #hpp_type_extensions

                        // This is useful for easily implementing the move constructor and assignment operators:
                        void swap(#pascal_case_ident& other) noexcept {
                            // Swap tags:
                            auto tag_temp = this->_tag;
                            this->_tag = other._tag;
                            other._tag = tag_temp;

                            // Swap data:
                            this->_data.swap(other._data);
                        }

                        #(#methods_hpp)*

                    private:
                        detail::#tag_typename _tag;
                        detail::#data_typename _data;
                    };
                }
            }
        };

        let cpp_methods = methods.iter().map(|m| m.to_cpp_tokens(&pascal_case_ident));
        let cpp = quote! {
            #cpp_includes

            #(#constants_cpp;)*

            namespace rerun {
                namespace #namespace_ident {
                    #(#cpp_methods)*
                }
            }
        };

        Self { hpp, cpp }
    }
}

fn single_field_constructor_methods(
    obj: &Object,
    hpp_includes: &mut Includes,
    type_ident: &Ident,
    objects: &Objects,
) -> Vec<Method> {
    let mut methods = Vec::new();

    // Single-field struct - it is a newtype wrapper.
    // Create a implicit constructor and assignment from its own field-type.
    let obj_field = &obj.fields[0];

    let field_ident = format_ident!("{}", obj_field.name);
    let param_ident = format_ident!("_{}", obj_field.name);

    if let Type::Array { elem_type, length } = &obj_field.typ {
        // Reminder: Arrays can't be passed by value, they decay to pointers. So we pass by reference!
        // (we could pass by pointer, but if an extension wants to add smaller array constructor these would be ambiguous then!)
        let length_quoted = quote_integer(length);
        let element_type = quote_element_type(hpp_includes, elem_type);
        let element_assignments = (0..*length).map(|i| {
            let i = quote_integer(i);
            quote!(#param_ident[#i])
        });
        methods.push(Method {
            declaration: MethodDeclaration::constructor(quote! {
                #type_ident(const #element_type (&#param_ident)[#length_quoted]) : #field_ident{#(#element_assignments),*}
            }),
            ..Method::default()
        });

        // No assignment operator for arrays since c arrays aren't typically assignable anyways.
        // Note that creating an std::array overload would make initializer_list based construction ambiguous.
        // Assignment operator for std::array could make sense though?
    } else {
        // Pass by value:
        // If it was a temporary it gets moved into the value and then moved again into the field.
        // If it was a lvalue it gets copied into the value and then moved into the field.
        let parameter_declaration = quote_variable(hpp_includes, obj_field, &param_ident);
        hpp_includes.insert_system("utility"); // std::move
        methods.push(Method {
            declaration: MethodDeclaration::constructor(quote! {
                #type_ident(#parameter_declaration) : #field_ident(std::move(#param_ident))
            }),
            ..Method::default()
        });
        methods.push(Method {
            declaration: MethodDeclaration {
                is_static: false,
                return_type: quote!(#type_ident&),
                name_and_parameters: quote! {
                    operator=(#parameter_declaration)
                },
            },
            definition_body: quote! {
                #field_ident = std::move(#param_ident);
                return *this;
            },
            ..Method::default()
        });

        // If the field is a custom type as well which in turn has only a single field,
        // provide a constructor for that single field as well.
        //
        // Note that we previously we tried to do a general forwarding constructor via variadic templates,
        // but ran into some issues when init archetypes with initializer lists.
        if let Type::Object(field_type_fqname) = &obj_field.typ {
            let field_type_obj = &objects[field_type_fqname];
            if field_type_obj.fields.len() == 1 {
                let inner_field = &field_type_obj.fields[0];
                let arg_name = format_ident!("arg");

                if let Type::Array { elem_type, length } = &inner_field.typ {
                    // Reminder: Arrays can't be passed by value, they decay to pointers. So we pass by reference!
                    // (we could pass by pointer, but if an extension wants to add smaller array constructor these would be ambiguous then!)
                    let length_quoted = quote_integer(length);
                    let element_type = quote_element_type(hpp_includes, elem_type);
                    methods.push(Method {
                        declaration: MethodDeclaration::constructor(quote! {
                            #type_ident(const #element_type (&#arg_name)[#length_quoted]) : #field_ident(#arg_name)
                        }),
                        ..Method::default()
                    });
                } else {
                    let argument = quote_variable(hpp_includes, inner_field, &arg_name);
                    methods.push(Method {
                        declaration: MethodDeclaration::constructor(quote! {
                            #type_ident(#argument) : #field_ident(std::move(#arg_name))
                        }),
                        ..Method::default()
                    });
                }
            }
        }
    }

    methods
}

fn arrow_data_type_method(
    obj: &Object,
    objects: &Objects,
    hpp_includes: &mut Includes,
    cpp_includes: &mut Includes,
    hpp_declarations: &mut ForwardDecls,
) -> Method {
    hpp_includes.insert_system("memory"); // std::shared_ptr
    cpp_includes.insert_system("arrow/type_fwd.h");
    hpp_declarations.insert("arrow", ForwardDecl::Class(format_ident!("DataType")));

    let quoted_datatype = quote_arrow_data_type(
        &Type::Object(obj.fqname.clone()),
        objects,
        cpp_includes,
        true,
    );

    Method {
        docs: "Returns the arrow data type this type corresponds to.".into(),
        declaration: MethodDeclaration {
            is_static: true,
            return_type: quote! { const std::shared_ptr<arrow::DataType>& },
            name_and_parameters: quote! { arrow_datatype() },
        },
        definition_body: quote! {
            static const auto datatype = #quoted_datatype;
            return datatype;
        },
        inline: false,
    }
}

fn new_arrow_array_builder_method(
    obj: &Object,
    objects: &Objects,
    hpp_includes: &mut Includes,
    cpp_includes: &mut Includes,
    hpp_declarations: &mut ForwardDecls,
) -> Method {
    hpp_includes.insert_system("memory"); // std::shared_ptr
    cpp_includes.insert_system("arrow/builder.h");
    hpp_declarations.insert("arrow", ForwardDecl::Class(format_ident!("MemoryPool")));

    let builder_instantiation = quote_arrow_array_builder_type_instantiation(
        &Type::Object(obj.fqname.clone()),
        objects,
        cpp_includes,
        true,
    );
    let arrow_builder_type = arrow_array_builder_type_object(obj, objects, hpp_declarations);

    Method {
        docs: "Creates a new array builder with an array of this type.".into(),
        declaration: MethodDeclaration {
            is_static: true,
            return_type: quote! { Result<std::shared_ptr<arrow::#arrow_builder_type>> },
            name_and_parameters: quote!(new_arrow_array_builder(arrow::MemoryPool * memory_pool)),
        },
        definition_body: quote! {
            if (!memory_pool) {
                return Error(ErrorCode::UnexpectedNullArgument, "Memory pool is null.");
            }
            #NEWLINE_TOKEN
            #NEWLINE_TOKEN
            return Result(#builder_instantiation);
        },
        inline: false,
    }
}

fn fill_arrow_array_builder_method(
    obj: &Object,
    type_ident: &Ident,
    cpp_includes: &mut Includes,
    hpp_declarations: &mut ForwardDecls,
    objects: &Objects,
) -> Method {
    cpp_includes.insert_system("arrow/builder.h");

    let builder = format_ident!("builder");
    let arrow_builder_type = arrow_array_builder_type_object(obj, objects, hpp_declarations);

    let fill_builder =
        quote_fill_arrow_array_builder(type_ident, obj, objects, &builder, cpp_includes);

    Method {
        docs: "Fills an arrow array builder with an array of this type.".into(),
        declaration: MethodDeclaration {
            is_static: true,
            return_type: quote! { Error },
            // TODO(andreas): Pass in validity map.
            name_and_parameters: quote! {
                fill_arrow_array_builder(arrow::#arrow_builder_type* #builder, const #type_ident* elements, size_t num_elements)
            },
        },
        definition_body: quote! {
            if (!builder) {
                return Error(ErrorCode::UnexpectedNullArgument, "Passed array builder is null.");
            }
            if (!elements) {
                return Error(ErrorCode::UnexpectedNullArgument, "Cannot serialize null pointer to arrow array.");
            }
            #NEWLINE_TOKEN
            #NEWLINE_TOKEN
            #fill_builder
            #NEWLINE_TOKEN
            #NEWLINE_TOKEN
            return Error::ok();
        },
        inline: false,
    }
}

fn component_to_data_cell_method(
    type_ident: &Ident,
    hpp_includes: &mut Includes,
    cpp_includes: &mut Includes,
) -> Method {
    hpp_includes.insert_system("memory"); // std::shared_ptr
    hpp_includes.insert_rerun("data_cell.hpp");
    hpp_includes.insert_rerun("result.hpp");
    cpp_includes.insert_rerun("arrow.hpp"); // ipc_from_table
    cpp_includes.insert_system("arrow/table.h"); // Table::Make

    let todo_pool = quote_comment("TODO(andreas): Allow configuring the memory pool.");

    Method {
        docs: format!("Creates a Rerun DataCell from an array of {type_ident} components.").into(),
        declaration: MethodDeclaration {
            is_static: true,
            return_type: quote! { Result<rerun::DataCell> },
            name_and_parameters: quote! {
                to_data_cell(const #type_ident* instances, size_t num_instances)
            },
        },
        definition_body: quote! {
            #NEWLINE_TOKEN
            #todo_pool
            arrow::MemoryPool* pool = arrow::default_memory_pool();
            #NEWLINE_TOKEN
            #NEWLINE_TOKEN
            auto builder_result = #type_ident::new_arrow_array_builder(pool);
            RR_RETURN_NOT_OK(builder_result.error);
            auto builder = std::move(builder_result.value);
            if (instances && num_instances > 0) {
                RR_RETURN_NOT_OK(#type_ident::fill_arrow_array_builder(
                    builder.get(),
                    instances,
                    num_instances
                ));
            }
            std::shared_ptr<arrow::Array> array;
            ARROW_RETURN_NOT_OK(builder->Finish(&array));
            #NEWLINE_TOKEN
            #NEWLINE_TOKEN
            auto schema = arrow::schema({arrow::field(
                #type_ident::NAME, // Unused, but should be the name of the field in the archetype if any.
                #type_ident::arrow_datatype(),
                false
            )});
            #NEWLINE_TOKEN
            #NEWLINE_TOKEN
            rerun::DataCell cell;
            cell.component_name = #type_ident::NAME;
            const auto ipc_result = rerun::ipc_from_table(*arrow::Table::Make(schema, {array}));
            RR_RETURN_NOT_OK(ipc_result.error);
            cell.buffer = std::move(ipc_result.value);
            #NEWLINE_TOKEN
            #NEWLINE_TOKEN
            return cell;
        },
        inline: false,
    }
}

fn archetype_to_data_cells(
    obj: &Object,
    hpp_includes: &mut Includes,
    cpp_includes: &mut Includes,
) -> Method {
    hpp_includes.insert_rerun("data_cell.hpp");
    hpp_includes.insert_rerun("result.hpp");
    hpp_includes.insert_rerun("arrow.hpp");
    hpp_includes.insert_system("vector"); // std::vector

    // TODO(andreas): Splats need to be handled separately.

    let num_fields = quote_integer(obj.fields.len());
    let push_cells = obj.fields.iter().map(|field| {
        let field_type_fqname = match &field.typ {
            Type::Vector { elem_type } => elem_type.fqname().unwrap(),
            Type::Object(fqname) => fqname,
            _ => unreachable!(
                "Archetypes are not expected to have any fields other than objects and vectors"
            ),
        };
        let field_type = quote_fqname_as_type_path(cpp_includes, field_type_fqname);
        let field_name = format_ident!("{}", field.name);

        if field.is_nullable {
            let to_data_cell = if field.typ.is_plural() {
                quote!(#field_type::to_data_cell(value.data(), value.size()))
            } else {
                quote!(#field_type::to_data_cell(&value, 1))
            };
            quote! {
                if (#field_name.has_value()) {
                    const auto& value  = #field_name.value();
                    const auto result = #to_data_cell;
                    if (result.is_err()) {
                        return result.error;
                    }
                    cells.emplace_back(std::move(result.value));
                }
            }
        } else {
            let to_data_cell = if field.typ.is_plural() {
                quote!(#field_type::to_data_cell(#field_name.data(), #field_name.size()))
            } else {
                quote!(#field_type::to_data_cell(&#field_name, 1))
            };
            quote! {
                {
                    const auto result = #to_data_cell;
                    if (result.is_err()) {
                        return result.error;
                    }
                    cells.emplace_back(std::move(result.value));
                }
            }
        }
    });

    let indicator_fqname = format!("rerun.components.{}Indicator", obj.name);
    Method {
        docs: "Creates a list of Rerun DataCell from this archetype.".into(),
        declaration: MethodDeclaration {
            is_static: false,
            return_type: quote!(Result<std::vector<rerun::DataCell>>),
            name_and_parameters: quote!(to_data_cells() const),
        },
        definition_body: quote! {
            std::vector<rerun::DataCell> cells;
            cells.reserve(#num_fields);
            #NEWLINE_TOKEN
            #NEWLINE_TOKEN
            #(#push_cells)*
            {
                const auto result =
                    create_indicator_component(#indicator_fqname, num_instances());
                if (result.is_err()) {
                    return result.error;
                }
                cells.emplace_back(std::move(result.value));
            }
            #NEWLINE_TOKEN
            #NEWLINE_TOKEN
            return cells;
        },
        inline: false,
    }
}

fn quote_fill_arrow_array_builder(
    type_ident: &Ident,
    obj: &Object,
    objects: &Objects,
    builder: &Ident,
    includes: &mut Includes,
) -> TokenStream {
    if obj.is_arrow_transparent() {
        let field = &obj.fields[0];
        if let Type::Object(fqname) = &field.typ {
            if field.is_nullable {
                quote! {
                    (void)num_elements;
                    return Error(ErrorCode::NotImplemented, "TODO(andreas) Handle nullable extensions");
                }
            } else {
                // Trivial forwarding to inner type.
                let quoted_fqname = quote_fqname_as_type_path(includes, fqname);
                quote! {
                    static_assert(sizeof(#quoted_fqname) == sizeof(#type_ident));
                    RR_RETURN_NOT_OK(#quoted_fqname::fill_arrow_array_builder(
                        builder, reinterpret_cast<const #quoted_fqname*>(elements), num_elements
                    ));
                }
            }
        } else {
            quote_append_field_to_builder(&obj.fields[0], builder, true, includes, objects)
        }
    } else {
        match obj.specifics {
            ObjectSpecifics::Struct => {
                let fill_fields = obj.fields.iter().enumerate().map(
                |(field_index, field)| {
                    let field_index = quote_integer(field_index);
                    let field_builder = format_ident!("field_builder");
                    let field_builder_type = arrow_array_builder_type(&field.typ, objects);
                    let field_append = quote_append_field_to_builder(field, &field_builder, false, includes, objects);
                    quote! {
                        {
                            auto #field_builder = static_cast<arrow::#field_builder_type*>(builder->field_builder(#field_index));
                            #field_append
                        }
                    }
                },
            );

                quote! {
                    #(#fill_fields)*
                    #NEWLINE_TOKEN
                    ARROW_RETURN_NOT_OK(builder->AppendValues(static_cast<int64_t>(num_elements), nullptr));
                }
            }
            ObjectSpecifics::Union { .. } => {
                let variant_builder = format_ident!("variant_builder");
                let tag_name = format_ident!("{}Tag", type_ident);

                let tag_cases = obj.fields
                .iter()
                .map(|variant| {
                    let arrow_builder_type = arrow_array_builder_type(&variant.typ, objects);
                    let variant_name = format_ident!("{}", variant.name);

                    let variant_append = if variant.typ.is_plural() {
                        quote! {
                            (void)#variant_builder;
                            return Error(ErrorCode::NotImplemented, "TODO(andreas): list types in unions are not yet supported");
                        }
                    } else {
                        let variant_accessor = quote!(union_instance._data);
                        quote_append_single_field_to_builder(variant, &variant_builder, &variant_accessor, includes)
                    };

                    quote! {
                        case detail::#tag_name::#variant_name: {
                            auto #variant_builder = static_cast<arrow::#arrow_builder_type*>(variant_builder_untyped);
                            #variant_append
                            break;
                        }
                    }
                });

                quote! {
                    #NEWLINE_TOKEN
                    ARROW_RETURN_NOT_OK(#builder->Reserve(static_cast<int64_t>(num_elements)));
                    for (size_t elem_idx = 0; elem_idx < num_elements; elem_idx += 1) {
                        const auto& union_instance = elements[elem_idx];
                        ARROW_RETURN_NOT_OK(#builder->Append(static_cast<int8_t>(union_instance._tag)));
                        #NEWLINE_TOKEN
                        #NEWLINE_TOKEN
                        auto variant_index = static_cast<int>(union_instance._tag);
                        auto variant_builder_untyped = builder->child_builder(variant_index).get();
                        #NEWLINE_TOKEN
                        #NEWLINE_TOKEN
                        switch (union_instance._tag) {
                            case detail::#tag_name::NONE: {
                                ARROW_RETURN_NOT_OK(variant_builder_untyped->AppendNull());
                                break;
                            }
                            #(#tag_cases)*
                        }
                    }
                }
            }
        }
    }
}

fn quote_append_field_to_builder(
    field: &ObjectField,
    builder: &Ident,
    is_transparent: bool,
    includes: &mut Includes,
    objects: &Objects,
) -> TokenStream {
    let field_name = format_ident!("{}", field.name);

    if let Some(elem_type) = field.typ.plural_inner() {
        let value_builder = format_ident!("value_builder");
        let value_builder_type = arrow_array_builder_type(&elem_type.clone().into(), objects);

        if !field.is_nullable
            && matches!(field.typ, Type::Array { .. })
            && elem_type.has_default_destructor(objects)
        {
            // Optimize common case: Trivial batch of transparent fixed size elements.
            let field_accessor = quote!(elements[0].#field_name);
            let num_items_per_value = quote_num_items_per_value(&field.typ, &field_accessor);
            let field_ptr_accessor = quote_field_ptr_access(&field.typ, field_accessor);
            quote! {
                auto #value_builder = static_cast<arrow::#value_builder_type*>(#builder->value_builder());
                #NEWLINE_TOKEN #NEWLINE_TOKEN
                ARROW_RETURN_NOT_OK(#builder->AppendValues(static_cast<int64_t>(num_elements)));
                static_assert(sizeof(elements[0].#field_name) == sizeof(elements[0]));
                ARROW_RETURN_NOT_OK(#value_builder->AppendValues(
                    #field_ptr_accessor,
                    static_cast<int64_t>(num_elements * #num_items_per_value),
                    nullptr)
                );
            }
        } else {
            let value_reserve_factor = match &field.typ {
                Type::Vector { .. } => {
                    if field.is_nullable {
                        1
                    } else {
                        2
                    }
                }
                Type::Array { length, .. } => *length,
                _ => unreachable!(),
            };
            let value_reserve_factor = quote_integer(value_reserve_factor);

            let setup = quote! {
                auto #value_builder = static_cast<arrow::#value_builder_type*>(#builder->value_builder());
                ARROW_RETURN_NOT_OK(#builder->Reserve(static_cast<int64_t>(num_elements)));
                ARROW_RETURN_NOT_OK(#value_builder->Reserve(static_cast<int64_t>(num_elements * #value_reserve_factor)));
                #NEWLINE_TOKEN #NEWLINE_TOKEN
            };

            let value_accessor = if field.is_nullable {
                quote!(element.#field_name.value())
            } else {
                quote!(element.#field_name)
            };

            let append_value = quote_append_single_value_to_builder(
                &field.typ,
                &value_builder,
                value_accessor,
                includes,
            );

            if field.is_nullable {
                quote! {
                    #setup
                    for (size_t elem_idx = 0; elem_idx < num_elements; elem_idx += 1) {
                        const auto& element = elements[elem_idx];
                        if (element.#field_name.has_value()) {
                            ARROW_RETURN_NOT_OK(#builder->Append());
                            #append_value
                        } else {
                            ARROW_RETURN_NOT_OK(#builder->AppendNull());
                        }
                    }
                }
            } else {
                quote! {
                    #setup
                    for (size_t elem_idx = 0; elem_idx < num_elements; elem_idx += 1) {
                        const auto& element = elements[elem_idx];
                        ARROW_RETURN_NOT_OK(#builder->Append());
                        #append_value
                    }
                }
            }
        }
    } else if !field.is_nullable && is_transparent && field.typ.has_default_destructor(objects) {
        // Trivial optimization: If this is the only field of this type and it's a trivial field (not array/string/blob),
        // we can just pass the whole array as-is!
        let field_ptr_accessor = quote_field_ptr_access(&field.typ, quote!(elements->#field_name));
        quote! {
            static_assert(sizeof(*elements) == sizeof(elements->#field_name));
            ARROW_RETURN_NOT_OK(#builder->AppendValues(#field_ptr_accessor, static_cast<int64_t>(num_elements)));
        }
    } else {
        let element_accessor = quote!(elements[elem_idx]);
        let single_append =
            quote_append_single_field_to_builder(field, builder, &element_accessor, includes);
        quote! {
            ARROW_RETURN_NOT_OK(#builder->Reserve(static_cast<int64_t>(num_elements)));
            for (size_t elem_idx = 0; elem_idx < num_elements; elem_idx += 1) {
                #single_append
            }
        }
    }
}

fn quote_append_single_field_to_builder(
    field: &ObjectField,
    builder: &Ident,
    element_accessor: &TokenStream,
    includes: &mut Includes,
) -> TokenStream {
    let field_name = format_ident!("{}", crate::to_snake_case(&field.name));
    let value_access = if field.is_nullable {
        quote!(element.#field_name.value())
    } else {
        quote!(#element_accessor.#field_name)
    };

    let append_value =
        quote_append_single_value_to_builder(&field.typ, builder, value_access, includes);

    if field.is_nullable {
        quote! {
            const auto& element = #element_accessor;
            if (element.#field_name.has_value()) {
                #append_value
            } else {
                ARROW_RETURN_NOT_OK(#builder->AppendNull());
            }
        }
    } else {
        quote! {
            #append_value
        }
    }
}

/// Appends a single value to an arrow array builder.
///
/// If the value is an array/vector, it will try to append the batch in one go.
/// Note that in that case this does *not* take care of the array/vector builder itself, just the underlying value builder.
fn quote_append_single_value_to_builder(
    typ: &Type,
    value_builder: &Ident,
    value_access: TokenStream,
    includes: &mut Includes,
) -> TokenStream {
    match &typ {
        Type::UInt8
        | Type::UInt16
        | Type::UInt32
        | Type::UInt64
        | Type::Int8
        | Type::Int16
        | Type::Int32
        | Type::Int64
        | Type::Bool
        | Type::Float16
        | Type::Float32
        | Type::Float64
        | Type::String => {
            quote!(ARROW_RETURN_NOT_OK(#value_builder->Append(#value_access));)
        }
        Type::Array { elem_type, .. } | Type::Vector { elem_type } => {
            let num_items_per_element = quote_num_items_per_value(typ, &value_access);

            match elem_type {
                ElementType::UInt8
                | ElementType::UInt16
                | ElementType::UInt32
                | ElementType::UInt64
                | ElementType::Int8
                | ElementType::Int16
                | ElementType::Int32
                | ElementType::Int64
                | ElementType::Bool
                | ElementType::Float16
                | ElementType::Float32
                | ElementType::Float64 => {
                    let field_ptr_accessor = quote_field_ptr_access(typ, value_access);
                    quote! {
                        ARROW_RETURN_NOT_OK(#value_builder->AppendValues(#field_ptr_accessor, static_cast<int64_t>(#num_items_per_element), nullptr));
                    }
                }
                ElementType::String => {
                    quote! {
                        for (size_t item_idx = 0; item_idx < #num_items_per_element; item_idx += 1) {
                            ARROW_RETURN_NOT_OK(#value_builder->Append(#value_access[item_idx]));
                        }
                    }
                }
                ElementType::Object(fqname) => {
                    let fqname = quote_fqname_as_type_path(includes, fqname);
                    let field_ptr_accessor = quote_field_ptr_access(typ, value_access);
                    quote! {
                        if (#field_ptr_accessor) {
                            RR_RETURN_NOT_OK(#fqname::fill_arrow_array_builder(#value_builder, #field_ptr_accessor, #num_items_per_element));
                        }
                    }
                }
            }
        }
        Type::Object(fqname) => {
            let fqname = quote_fqname_as_type_path(includes, fqname);
            quote!(RR_RETURN_NOT_OK(#fqname::fill_arrow_array_builder(#value_builder, &#value_access, 1));)
        }
    }
}

fn quote_num_items_per_value(typ: &Type, value_accessor: &TokenStream) -> TokenStream {
    match &typ {
        Type::Array { length, .. } => quote_integer(length),
        Type::Vector { .. } => quote!(#value_accessor.size()),
        _ => quote_integer(1),
    }
}

fn quote_field_ptr_access(typ: &Type, field_accessor: TokenStream) -> TokenStream {
    let (ptr_access, typ) = match typ {
        Type::Array { elem_type, .. } => (field_accessor, elem_type.clone().into()),
        Type::Vector { elem_type } => (quote!(#field_accessor.data()), elem_type.clone().into()),
        _ => (quote!(&#field_accessor), typ.clone()),
    };

    if typ == Type::Bool {
        // Bool needs a cast because arrow takes it as uint8_t.
        quote!(reinterpret_cast<const uint8_t*>(#ptr_access))
    } else {
        ptr_access
    }
}

/// e.g. `static Angle radians(float radians);` -> `auto angle = Angle::radians(radians);`
fn static_constructor_for_enum_type(
    objects: &Objects,
    hpp_includes: &mut Includes,
    obj_field: &ObjectField,
    pascal_case_ident: &Ident,
    tag_typename: &Ident,
) -> Method {
    let tag_ident = format_ident!("{}", obj_field.name);
    let snake_case_ident = format_ident!("{}", crate::to_snake_case(&obj_field.name));
    let docs = obj_field.docs.clone().into();

    let param_declaration = quote_variable(hpp_includes, obj_field, &snake_case_ident);
    let declaration = MethodDeclaration {
        is_static: true,
        return_type: quote!(#pascal_case_ident),
        name_and_parameters: quote!(#snake_case_ident(#param_declaration)),
    };

    if let Type::Array { elem_type, length } = &obj_field.typ {
        // We need special casing for constructing arrays:
        let length = proc_macro2::Literal::usize_unsuffixed(*length);

        let (element_assignment, typedef) = if elem_type.has_default_destructor(objects) {
            // Generate simpoler code for simple types:
            (
                quote! {
                    self._data.#snake_case_ident[i] = std::move(#snake_case_ident[i]);
                },
                quote!(),
            )
        } else {
            // We need to use placement-new since the union is in an uninitialized state here:
            hpp_includes.insert_system("new"); // placement-new
            let elem_type = quote_element_type(hpp_includes, elem_type);
            (
                quote! {
                    new (&self._data.#snake_case_ident[i]) #elem_type(std::move(#snake_case_ident[i]));
                },
                quote!(typedef #elem_type TypeAlias;),
            )
        };

        Method {
            docs,
            declaration,
            definition_body: quote! {
                #typedef
                #pascal_case_ident self;
                self._tag = detail::#tag_typename::#tag_ident;
                for (size_t i = 0; i < #length; i += 1) {
                    #element_assignment
                }
                return self;
            },
            inline: true,
        }
    } else if obj_field.typ.has_default_destructor(objects) {
        // Generate simpler code for simple types:
        Method {
            docs,
            declaration,
            definition_body: quote! {
                #pascal_case_ident self;
                self._tag = detail::#tag_typename::#tag_ident;
                self._data.#snake_case_ident = std::move(#snake_case_ident);
                return self;
            },
            inline: true,
        }
    } else {
        // We need to use placement-new since the union is in an uninitialized state here:
        hpp_includes.insert_system("new"); // placement-new
        let typedef_declaration =
            quote_variable(hpp_includes, obj_field, &format_ident!("TypeAlias"));
        Method {
            docs,
            declaration,
            definition_body: quote! {
                typedef #typedef_declaration;
                #pascal_case_ident self;
                self._tag = detail::#tag_typename::#tag_ident;
                new (&self._data.#snake_case_ident) TypeAlias(std::move(#snake_case_ident));
                return self;
            },
            inline: true,
        }
    }
}

fn quote_constants_header_and_cpp(
    obj: &Object,
    objects: &Objects,
    obj_type_ident: &Ident,
) -> (Vec<TokenStream>, Vec<TokenStream>) {
    let mut hpp = Vec::new();
    let mut cpp = Vec::new();
    match &obj.kind {
        ObjectKind::Component => {
            let legacy_fqname = objects[&obj.fqname]
                .try_get_attr::<String>(crate::ATTR_RERUN_LEGACY_FQNAME)
                .unwrap_or_else(|| obj.fqname.clone());

            let comment = quote_doc_comment("Name of the component, used for serialization.");
            hpp.push(quote! {
                #NEWLINE_TOKEN
                #NEWLINE_TOKEN
                #comment
                static const char* NAME
            });
            cpp.push(quote!(const char* #obj_type_ident::NAME = #legacy_fqname));
        }
        ObjectKind::Archetype | ObjectKind::Datatype => {}
    }

    (hpp, cpp)
}

fn are_types_disjoint(fields: &[ObjectField]) -> bool {
    let type_set: std::collections::HashSet<&Type> = fields.iter().map(|f| &f.typ).collect();
    type_set.len() == fields.len()
}

fn quote_variable_with_docstring(
    includes: &mut Includes,
    obj_field: &ObjectField,
    name: &syn::Ident,
) -> TokenStream {
    let quoted = quote_variable(includes, obj_field, name);

    let docstring = quote_docstrings(&obj_field.docs);

    let quoted = quote! {
        #docstring
        #quoted
    };

    quoted
}

fn quote_variable(
    includes: &mut Includes,
    obj_field: &ObjectField,
    name: &syn::Ident,
) -> TokenStream {
    if obj_field.is_nullable {
        includes.insert_system("optional");
        match &obj_field.typ {
            Type::UInt8 => quote! { std::optional<uint8_t> #name },
            Type::UInt16 => quote! { std::optional<uint16_t> #name },
            Type::UInt32 => quote! { std::optional<uint32_t> #name },
            Type::UInt64 => quote! { std::optional<uint64_t> #name },
            Type::Int8 => quote! { std::optional<int8_t> #name },
            Type::Int16 => quote! { std::optional<int16_t> #name },
            Type::Int32 => quote! { std::optional<int32_t> #name },
            Type::Int64 => quote! { std::optional<int64_t> #name },
            Type::Bool => quote! { std::optional<bool> #name },
            Type::Float16 => unimplemented!("float16 not yet implemented for C++"),
            Type::Float32 => quote! { std::optional<float> #name },
            Type::Float64 => quote! { std::optional<double> #name },
            Type::String => {
                includes.insert_system("string");
                quote! { std::optional<std::string> #name }
            }
            Type::Array { .. } => {
                unimplemented!(
                    "Optional fixed-size array not yet implemented in C++. {:#?}",
                    obj_field.typ
                )
            }
            Type::Vector { elem_type } => {
                let elem_type = quote_element_type(includes, elem_type);
                includes.insert_system("vector");
                quote! { std::optional<std::vector<#elem_type>> #name }
            }
            Type::Object(fqname) => {
                let type_name = quote_fqname_as_type_path(includes, fqname);
                quote! { std::optional<#type_name> #name }
            }
        }
    } else {
        match &obj_field.typ {
            Type::UInt8 => quote! { uint8_t #name },
            Type::UInt16 => quote! { uint16_t #name },
            Type::UInt32 => quote! { uint32_t #name },
            Type::UInt64 => quote! { uint64_t #name },
            Type::Int8 => quote! { int8_t #name },
            Type::Int16 => quote! { int16_t #name },
            Type::Int32 => quote! { int32_t #name },
            Type::Int64 => quote! { int64_t #name },
            Type::Bool => quote! { bool #name },
            Type::Float16 => unimplemented!("float16 not yet implemented for C++"),
            Type::Float32 => quote! { float #name },
            Type::Float64 => quote! { double #name },
            Type::String => {
                includes.insert_system("string");
                quote! { std::string #name }
            }
            Type::Array { elem_type, length } => {
                let elem_type = quote_element_type(includes, elem_type);
                let length = proc_macro2::Literal::usize_unsuffixed(*length);

                quote! { #elem_type #name[#length] }
            }
            Type::Vector { elem_type } => {
                let elem_type = quote_element_type(includes, elem_type);
                includes.insert_system("vector");
                quote! { std::vector<#elem_type> #name }
            }
            Type::Object(fqname) => {
                let type_name = quote_fqname_as_type_path(includes, fqname);
                quote! { #type_name #name }
            }
        }
    }
}

fn quote_element_type(includes: &mut Includes, typ: &ElementType) -> TokenStream {
    match typ {
        ElementType::UInt8 => quote! { uint8_t },
        ElementType::UInt16 => quote! { uint16_t },
        ElementType::UInt32 => quote! { uint32_t },
        ElementType::UInt64 => quote! { uint64_t },
        ElementType::Int8 => quote! { int8_t },
        ElementType::Int16 => quote! { int16_t },
        ElementType::Int32 => quote! { int32_t },
        ElementType::Int64 => quote! { int64_t },
        ElementType::Bool => quote! { bool },
        ElementType::Float16 => unimplemented!("float16 not yet implemented for C++"),
        ElementType::Float32 => quote! { float },
        ElementType::Float64 => quote! { double },
        ElementType::String => {
            includes.insert_system("string");
            quote! { std::string }
        }
        ElementType::Object(fqname) => quote_fqname_as_type_path(includes, fqname),
    }
}

fn quote_fqname_as_type_path(includes: &mut Includes, fqname: &str) -> TokenStream {
    includes.insert_rerun_type(fqname);

    let fqname = fqname
        .replace(".testing", "")
        .replace('.', "::")
        .replace("crate", "rerun");

    let expr: syn::TypePath = syn::parse_str(&fqname).unwrap();
    quote!(#expr)
}

fn quote_docstrings(docs: &Docs) -> TokenStream {
    let lines = crate::codegen::get_documentation(docs, &["cpp", "c++"]);
    let quoted_lines = lines.iter().map(|docstring| quote_doc_comment(docstring));
    quote! {
        #NEWLINE_TOKEN
        #(#quoted_lines)*
    }
}

fn quote_integer<T: std::fmt::Display>(t: T) -> TokenStream {
    let t = syn::LitInt::new(&t.to_string(), proc_macro2::Span::call_site());
    quote!(#t)
}

fn quote_arrow_data_type(
    typ: &Type,
    objects: &Objects,
    includes: &mut Includes,
    is_top_level_type: bool,
) -> TokenStream {
    match typ {
        Type::Int8 => quote!(arrow::int8()),
        Type::Int16 => quote!(arrow::int16()),
        Type::Int32 => quote!(arrow::int32()),
        Type::Int64 => quote!(arrow::int64()),
        Type::UInt8 => quote!(arrow::uint8()),
        Type::UInt16 => quote!(arrow::uint16()),
        Type::UInt32 => quote!(arrow::uint32()),
        Type::UInt64 => quote!(arrow::uint64()),
        Type::Float16 => quote!(arrow::float16()),
        Type::Float32 => quote!(arrow::float32()),
        Type::Float64 => quote!(arrow::float64()),
        Type::String => quote!(arrow::utf8()),
        Type::Bool => quote!(arrow::boolean()),

        Type::Vector { elem_type } => {
            let quoted_field = quote_arrow_elem_type(elem_type, objects, includes);
            quote!(arrow::list(#quoted_field))
        }

        Type::Array { elem_type, length } => {
            let quoted_field = quote_arrow_elem_type(elem_type, objects, includes);
            let quoted_length = quote_integer(length);
            quote!(arrow::fixed_size_list(#quoted_field, #quoted_length))
        }

        Type::Object(fqname) => {
            // TODO(andreas): We're no`t emitting the actual extension types here yet which is why we're skipping the extension type at top level.
            // Currently, we wrap only Components in extension types but this is done in `rerun_c`.
            // In the future we'll add the extension type here to the schema.
            let obj = &objects[fqname];
            if !is_top_level_type {
                // If we're not at the top level, we should have already a `arrow_datatype` method that we can relay to.
                let quoted_fqname = quote_fqname_as_type_path(includes, fqname);
                quote!(#quoted_fqname::arrow_datatype())
            } else if obj.is_arrow_transparent() {
                quote_arrow_data_type(&obj.fields[0].typ, objects, includes, false)
            } else {
                let quoted_fields = obj
                    .fields
                    .iter()
                    .map(|field| quote_arrow_field_type(field, objects, includes));

                match &obj.specifics {
                    ObjectSpecifics::Union { .. } => {
                        quote! {
                            arrow::dense_union({
                                arrow::field("_null_markers", arrow::null(), true, nullptr), #(#quoted_fields,)*
                            })
                        }
                    }
                    ObjectSpecifics::Struct => {
                        quote!(arrow::struct_({ #(#quoted_fields,)* }))
                    }
                }
            }
        }
    }
}

fn quote_arrow_field_type(
    field: &ObjectField,
    objects: &Objects,
    includes: &mut Includes,
) -> TokenStream {
    let name = &field.name;
    let datatype = quote_arrow_data_type(&field.typ, objects, includes, false);
    let is_nullable = field.is_nullable;

    quote! {
        arrow::field(#name, #datatype, #is_nullable)
    }
}

fn quote_arrow_elem_type(
    elem_type: &ElementType,
    objects: &Objects,
    includes: &mut Includes,
) -> TokenStream {
    let typ: Type = elem_type.clone().into();
    let datatype = quote_arrow_data_type(&typ, objects, includes, false);

    quote! {
        arrow::field("item", #datatype, false)
    }
}
