use criterion::{black_box, criterion_group, criterion_main, Criterion};
use doido_view::{TeraEngine, TemplateEngine};
use serde_json::json;
use std::fs;

fn tera_render(c: &mut Criterion) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("pages/show.html.tera");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        r#"<!DOCTYPE html>
<html><body><h1>{{ title }}</h1>
<ul>{% for item in items %}<li>{{ item }}</li>{% endfor %}</ul>
</body></html>"#,
    )
    .unwrap();

    let engine = TeraEngine::new(dir.path().to_str().unwrap()).unwrap();
    let ctx = json!({
        "title": "Benchmark",
        "items": (0..20).map(|i| format!("item-{i}")).collect::<Vec<_>>(),
    });

    c.bench_function("tera_render", |b| {
        b.iter(|| {
            let html = engine
                .render(black_box("pages/show"), black_box(&ctx))
                .unwrap();
            black_box(html);
        });
    });
}

criterion_group!(benches, tera_render);
criterion_main!(benches);
