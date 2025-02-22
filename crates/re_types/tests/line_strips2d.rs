use std::collections::HashMap;

use re_types::{
    archetypes::LineStrips2D,
    components::{ClassId, Color, DrawOrder, InstanceKey, Label, LineStrip2D, Radius},
    Archetype as _,
};

#[test]
fn roundtrip() {
    let expected = LineStrips2D {
        #[rustfmt::skip]
        strips: vec![
            LineStrip2D::from_iter([[0., 0.], [2., 1.], [4., -1.], [6., 0.]]), //
            LineStrip2D::from_iter([[0., 3.], [1., 4.], [2., 2.], [3., 4.], [4., 2.], [5., 4.], [6., 3.]]), //
        ],
        radii: Some(vec![
            Radius(42.0), //
            Radius(43.0),
        ]),
        colors: Some(vec![
            Color::from_unmultiplied_rgba(0xAA, 0x00, 0x00, 0xCC), //
            Color::from_unmultiplied_rgba(0x00, 0xBB, 0x00, 0xDD),
        ]),
        labels: Some(vec![
            Label("hello".into()),  //
            Label("friend".into()), //
        ]),
        draw_order: Some(DrawOrder(300.0)),
        class_ids: Some(vec![
            ClassId::from(126), //
            ClassId::from(127), //
        ]),
        instance_keys: Some(vec![
            InstanceKey(u64::MAX - 1), //
            InstanceKey(u64::MAX),
        ]),
    };

    #[rustfmt::skip]
    let strips = [
        [[0., 0.], [2., 1.], [4., -1.], [6., 0.]].to_vec(),
        [[0., 3.], [1., 4.], [2., 2.], [3., 4.], [4., 2.], [5., 4.], [6., 3.]].to_vec(),
    ];
    let arch = LineStrips2D::new(strips)
        .with_radii([42.0, 43.0])
        .with_colors([0xAA0000CC, 0x00BB00DD])
        .with_labels(["hello", "friend"])
        .with_draw_order(300.0)
        .with_class_ids([126, 127])
        .with_instance_keys([u64::MAX - 1, u64::MAX]);
    similar_asserts::assert_eq!(expected, arch);

    let expected_extensions: HashMap<_, _> = [
        ("points", vec!["rerun.components.LineStrip2D"]),
        ("radii", vec!["rerun.components.Radius"]),
        ("colors", vec!["rerun.components.Color"]),
        ("labels", vec!["rerun.components.Label"]),
        ("draw_order", vec!["rerun.components.DrawOrder"]),
        ("class_ids", vec!["rerun.components.ClassId"]),
        ("keypoint_ids", vec!["rerun.components.KeypointId"]),
        ("instance_keys", vec!["rerun.components.InstanceKey"]),
    ]
    .into();

    eprintln!("arch = {arch:#?}");
    let serialized = arch.to_arrow();
    for (field, array) in &serialized {
        // NOTE: Keep those around please, very useful when debugging.
        // eprintln!("field = {field:#?}");
        // eprintln!("array = {array:#?}");
        eprintln!("{} = {array:#?}", field.name);

        // TODO(cmc): Re-enable extensions and these assertions once `arrow2-convert`
        // has been fully replaced.
        if false {
            util::assert_extensions(
                &**array,
                expected_extensions[field.name.as_str()].as_slice(),
            );
        }
    }

    let deserialized = LineStrips2D::try_from_arrow(serialized).unwrap();
    similar_asserts::assert_eq!(expected, deserialized);
}

mod util;
