// Copyright 2019-2023 Parity Technologies (UK) Ltd.
// This file is dual-licensed as Apache-2.0 or GPL-3.0.
// see LICENSE for license details.

use frame_metadata::{
    v15::{ExtrinsicMetadata, RuntimeMetadataV15},
    RuntimeMetadataPrefixed,
};
use scale_info::{meta_type, IntoPortable, TypeInfo};
use subxt_codegen::{CratePath, DerivesRegistry, RuntimeGenerator, TypeSubstitutes};

fn generate_runtime_interface_from_metadata(metadata: RuntimeMetadataPrefixed) -> String {
    // Generate a runtime interface from the provided metadata.
    let generator = RuntimeGenerator::new(metadata);
    let item_mod = syn::parse_quote!(
        pub mod api {}
    );
    let crate_path = CratePath::default();
    let derives = DerivesRegistry::with_default_derives(&crate_path);
    let type_substitutes = TypeSubstitutes::with_default_substitutes(&crate_path);
    generator
        .generate_runtime(item_mod, derives, type_substitutes, crate_path, false)
        .expect("API generation must be valid")
        .to_string()
}

fn generate_runtime_interface_with_type_registry<F>(f: F) -> String
where
    F: Fn(&mut scale_info::Registry),
{
    #[derive(TypeInfo)]
    struct Runtime;
    #[derive(TypeInfo)]
    enum RuntimeCall {}
    #[derive(TypeInfo)]
    enum RuntimeEvent {}
    #[derive(TypeInfo)]
    pub enum DispatchError {}

    // We need these types for codegen to work:
    let mut registry = scale_info::Registry::new();
    let ty = registry.register_type(&meta_type::<Runtime>());
    registry.register_type(&meta_type::<RuntimeCall>());
    registry.register_type(&meta_type::<RuntimeEvent>());
    registry.register_type(&meta_type::<DispatchError>());

    // Allow custom types to be added for testing:
    f(&mut registry);

    let extrinsic = ExtrinsicMetadata {
        ty: meta_type::<()>(),
        version: 0,
        signed_extensions: vec![],
    }
    .into_portable(&mut registry);
    let metadata = RuntimeMetadataV15 {
        types: registry.into(),
        pallets: Vec::new(),
        extrinsic,
        ty,
        apis: vec![],
    };

    let metadata = RuntimeMetadataPrefixed::from(metadata);
    generate_runtime_interface_from_metadata(metadata)
}

#[test]
fn dupe_types_do_not_overwrite_each_other() {
    let interface = generate_runtime_interface_with_type_registry(|registry| {
        // Now we duplicate some types with same type info. We need two unique types here,
        // and can't just add one type to the registry twice, because the registry knows if
        // type IDs are the same.
        enum Foo {}
        impl TypeInfo for Foo {
            type Identity = Self;
            fn type_info() -> scale_info::Type {
                scale_info::Type::builder()
                    .path(scale_info::Path::new("DuplicateType", "dupe_mod"))
                    .variant(
                        scale_info::build::Variants::new()
                            .variant("FirstDupeTypeVariant", |builder| builder.index(0)),
                    )
            }
        }
        enum Bar {}
        impl TypeInfo for Bar {
            type Identity = Self;
            fn type_info() -> scale_info::Type {
                scale_info::Type::builder()
                    .path(scale_info::Path::new("DuplicateType", "dupe_mod"))
                    .variant(
                        scale_info::build::Variants::new()
                            .variant("SecondDupeTypeVariant", |builder| builder.index(0)),
                    )
            }
        }

        registry.register_type(&meta_type::<Foo>());
        registry.register_type(&meta_type::<Bar>());
    });

    assert!(interface.contains("DuplicateType"));
    assert!(interface.contains("FirstDupeTypeVariant"));

    assert!(interface.contains("DuplicateType2"));
    assert!(interface.contains("SecondDupeTypeVariant"));
}

#[test]
fn generic_types_overwrite_each_other() {
    let interface = generate_runtime_interface_with_type_registry(|registry| {
        // If we have two types mentioned in the registry that have generic params,
        // only one type will be output (the codegen assumes that the generic param will disambiguate)
        enum Foo {}
        impl TypeInfo for Foo {
            type Identity = Self;
            fn type_info() -> scale_info::Type {
                scale_info::Type::builder()
                    .path(scale_info::Path::new("DuplicateType", "dupe_mod"))
                    .type_params([scale_info::TypeParameter::new("T", Some(meta_type::<u8>()))])
                    .variant(scale_info::build::Variants::new())
            }
        }
        enum Bar {}
        impl TypeInfo for Bar {
            type Identity = Self;
            fn type_info() -> scale_info::Type {
                scale_info::Type::builder()
                    .path(scale_info::Path::new("DuplicateType", "dupe_mod"))
                    .type_params([scale_info::TypeParameter::new("T", Some(meta_type::<u8>()))])
                    .variant(scale_info::build::Variants::new())
            }
        }

        registry.register_type(&meta_type::<Foo>());
        registry.register_type(&meta_type::<Bar>());
    });

    assert!(interface.contains("DuplicateType"));
    // We do _not_ expect this to exist, since a generic is present on the type:
    assert!(!interface.contains("DuplicateType2"));
}
