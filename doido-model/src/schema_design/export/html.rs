//! Self-contained HTML ER diagram renderer.

use std::fmt::Write as _;
use std::path::Path;

use doido_core::Result;

use crate::schema_design::model::{ColumnDesign, ForeignKeyDesign, SchemaDesign, TableDesign};

/// Render the schema as a self-contained HTML document.
pub fn export_html(schema: &SchemaDesign) -> Result<String> {
    let json = serde_json::to_string(schema)
        .map_err(|e| doido_core::anyhow::anyhow!("schema json encode failed: {e}"))?;

    let mut body = String::new();
    writeln!(body, r#"<div id="diagram" class="diagram">"#)?;
    writeln!(body, r#"  <svg id="relations" class="relations"></svg>"#)?;
    writeln!(body, r#"  <div id="tables" class="tables-grid">"#)?;

    for table in schema.sorted_tables() {
        render_table(&mut body, table)?;
    }

    writeln!(body, "  </div>")?;
    writeln!(body, "</div>")?;
    writeln!(
        body,
        r#"<div id="tooltip" class="tooltip" role="tooltip" hidden></div>"#
    )?;

    let mut html = String::new();
    writeln!(html, "<!DOCTYPE html>")?;
    writeln!(html, r#"<html lang="en">"#)?;
    writeln!(html, "<head>")?;
    writeln!(html, r#"  <meta charset="utf-8">"#)?;
    writeln!(
        html,
        r#"  <meta name="viewport" content="width=device-width, initial-scale=1">"#
    )?;
    writeln!(html, "  <title>Database ER Diagram</title>")?;
    writeln!(html, "  <style>{STYLES}</style>")?;
    writeln!(html, "</head>")?;
    writeln!(html, "<body>")?;
    writeln!(html, "  <header class=\"page-header\">")?;
    writeln!(html, "    <h1>Entity-Relationship Diagram</h1>")?;
    writeln!(
        html,
        "    <p>Hover columns and table headers for types, constraints, and indexes.</p>"
    )?;
    writeln!(html, "  </header>")?;
    html.push_str(&body);
    writeln!(
        html,
        r#"  <script type="application/json" id="doido-schema-design">{json}</script>"#
    )?;
    writeln!(html, "  <script>{SCRIPT}</script>")?;
    writeln!(html, "</body>")?;
    writeln!(html, "</html>")?;

    Ok(html)
}

/// Write the HTML ER diagram to `path`, creating parent directories if needed.
pub fn write_html(schema: &SchemaDesign, path: impl AsRef<Path>) -> Result<()> {
    let html = export_html(schema)?;
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|e| doido_core::anyhow::anyhow!("create {}: {e}", parent.display()))?;
        }
    }
    std::fs::write(path, html)
        .map_err(|e| doido_core::anyhow::anyhow!("write {}: {e}", path.display()))
}

fn render_table(out: &mut String, table: &TableDesign) -> Result<()> {
    let table_tooltip = html_escape(&table_tooltip(table));
    writeln!(
        out,
        r#"    <div class="table-card" data-table="{}" tabindex="0">"#,
        html_escape(&table.name)
    )?;
    writeln!(
        out,
        r#"      <div class="table-header" data-tooltip="{table_tooltip}">{name}</div>"#,
        name = html_escape(&table.name)
    )?;
    writeln!(out, r#"      <div class="table-columns">"#)?;
    for col in &table.columns {
        render_column(out, col)?;
    }
    writeln!(out, "      </div>")?;
    writeln!(out, "    </div>")?;
    Ok(())
}

fn render_column(out: &mut String, col: &ColumnDesign) -> Result<()> {
    let tooltip = html_escape(&column_tooltip(col));
    write!(
        out,
        r#"        <div class="column-row" data-tooltip="{tooltip}">"#
    )?;
    write!(
        out,
        r#"<span class="col-name">{}</span>"#,
        html_escape(&col.name)
    )?;
    if col.primary_key {
        write!(out, r#"<span class="badge pk">PK</span>"#)?;
    }
    if col.foreign_key {
        write!(out, r#"<span class="badge fk">FK</span>"#)?;
    }
    if col.unique && !col.primary_key {
        write!(out, r#"<span class="badge uk">UK</span>"#)?;
    }
    writeln!(out, "</div>")?;
    Ok(())
}

fn column_tooltip(col: &ColumnDesign) -> String {
    let mut lines = vec![
        format!("type: {}", col.raw_type),
        format!("abstract: {:?}", col.abstract_type),
        format!("nullable: {}", col.nullable),
    ];
    if let Some(default) = &col.default {
        lines.push(format!("default: {default}"));
    }
    if col.unique {
        lines.push("unique: true".to_string());
    }
    if col.primary_key {
        lines.push("primary key".to_string());
    }
    if col.foreign_key {
        lines.push("foreign key".to_string());
    }
    lines.join("\n")
}

fn table_tooltip(table: &TableDesign) -> String {
    let mut lines = vec![format!("table: {}", table.name)];
    if let Some(schema) = &table.schema {
        lines.push(format!("schema: {schema}"));
    }
    if !table.primary_key.columns.is_empty() {
        lines.push(format!(
            "primary key: {}{}",
            table.primary_key.columns.join(", "),
            if table.primary_key.autoincrement {
                " (autoincrement)"
            } else {
                ""
            }
        ));
    }
    if !table.indexes.is_empty() {
        lines.push("indexes:".to_string());
        for idx in &table.indexes {
            lines.push(format!(
                "  - {} ({}){}",
                idx.name,
                idx.columns.join(", "),
                if idx.unique { " unique" } else { "" }
            ));
        }
    }
    if !table.constraints.is_empty() {
        lines.push("constraints:".to_string());
        for c in &table.constraints {
            let cols = if c.columns.is_empty() {
                String::new()
            } else {
                format!(" ({})", c.columns.join(", "))
            };
            let def = c
                .definition
                .as_ref()
                .map(|d| format!(": {d}"))
                .unwrap_or_default();
            lines.push(format!(
                "  - {:?}{}{}{}",
                c.kind,
                cols,
                def,
                c.name
                    .as_ref()
                    .map(|n| format!(" [{n}]"))
                    .unwrap_or_default()
            ));
        }
    }
    if !table.foreign_keys.is_empty() {
        lines.push("foreign keys:".to_string());
        for fk in &table.foreign_keys {
            lines.push(format!("  - {}", fk_tooltip(fk)));
        }
    }
    lines.join("\n")
}

fn fk_tooltip(fk: &ForeignKeyDesign) -> String {
    let mut s = format!(
        "{} -> {}.{}",
        fk.columns.join(", "),
        fk.referenced_table,
        fk.referenced_columns.join(", ")
    );
    if let Some(on_delete) = &fk.on_delete {
        s.push_str(&format!(" ON DELETE {on_delete}"));
    }
    if let Some(on_update) = &fk.on_update {
        s.push_str(&format!(" ON UPDATE {on_update}"));
    }
    s
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const STYLES: &str = r#"
:root {
  color-scheme: light dark;
  --bg: #f6f7f9;
  --card: #ffffff;
  --border: #d8dee6;
  --text: #1f2937;
  --muted: #64748b;
  --pk: #2563eb;
  --fk: #059669;
  --uk: #d97706;
  --line: #94a3b8;
}
@media (prefers-color-scheme: dark) {
  :root {
    --bg: #0f172a;
    --card: #1e293b;
    --border: #334155;
    --text: #e2e8f0;
    --muted: #94a3b8;
    --line: #64748b;
  }
}
* { box-sizing: border-box; }
body {
  margin: 0;
  font-family: system-ui, -apple-system, sans-serif;
  background: var(--bg);
  color: var(--text);
}
.page-header {
  padding: 1.5rem 2rem 0.5rem;
}
.page-header h1 { margin: 0 0 0.25rem; font-size: 1.5rem; }
.page-header p { margin: 0; color: var(--muted); font-size: 0.95rem; }
.diagram {
  position: relative;
  padding: 1rem 2rem 2rem;
  min-height: 400px;
}
.relations {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  overflow: visible;
}
.tables-grid {
  position: relative;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 1.25rem;
  z-index: 1;
}
.table-card {
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 1px 3px rgba(0,0,0,0.08);
  overflow: hidden;
}
.table-header {
  padding: 0.6rem 0.75rem;
  font-weight: 700;
  border-bottom: 1px solid var(--border);
  background: rgba(37, 99, 235, 0.08);
  cursor: help;
}
.table-columns { padding: 0.25rem 0; }
.column-row {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.35rem 0.75rem;
  font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
  font-size: 0.85rem;
  cursor: help;
}
.column-row:hover { background: rgba(148, 163, 184, 0.12); }
.col-name { flex: 1; }
.badge {
  font-size: 0.65rem;
  font-weight: 700;
  padding: 0.1rem 0.35rem;
  border-radius: 4px;
  letter-spacing: 0.03em;
}
.badge.pk { background: rgba(37, 99, 235, 0.15); color: var(--pk); }
.badge.fk { background: rgba(5, 150, 105, 0.15); color: var(--fk); }
.badge.uk { background: rgba(217, 119, 6, 0.15); color: var(--uk); }
.tooltip {
  position: fixed;
  z-index: 1000;
  max-width: 420px;
  padding: 0.65rem 0.75rem;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 6px;
  box-shadow: 0 8px 24px rgba(0,0,0,0.15);
  font-size: 0.8rem;
  line-height: 1.45;
  white-space: pre-wrap;
  pointer-events: none;
}
"#;

const SCRIPT: &str = r#"
(function () {
  const schema = JSON.parse(document.getElementById('doido-schema-design').textContent);
  const tooltip = document.getElementById('tooltip');
  const svg = document.getElementById('relations');

  function showTooltip(el, text) {
    tooltip.hidden = false;
    tooltip.textContent = text;
    positionTooltip(el);
  }
  function hideTooltip() {
    tooltip.hidden = true;
  }
  function positionTooltip(el) {
    const rect = el.getBoundingClientRect();
    let top = rect.bottom + 8;
    let left = rect.left;
    if (left + tooltip.offsetWidth > window.innerWidth - 12) {
      left = window.innerWidth - tooltip.offsetWidth - 12;
    }
    if (top + tooltip.offsetHeight > window.innerHeight - 12) {
      top = rect.top - tooltip.offsetHeight - 8;
    }
    tooltip.style.top = top + 'px';
    tooltip.style.left = left + 'px';
  }

  document.querySelectorAll('[data-tooltip]').forEach(function (el) {
    el.addEventListener('mouseenter', function () {
      showTooltip(el, el.getAttribute('data-tooltip'));
    });
    el.addEventListener('mousemove', function () {
      positionTooltip(el);
    });
    el.addEventListener('mouseleave', hideTooltip);
    el.addEventListener('focus', function () {
      showTooltip(el, el.getAttribute('data-tooltip'));
    });
    el.addEventListener('blur', hideTooltip);
  });

  function cardCenter(tableName) {
    const card = document.querySelector('.table-card[data-table="' + tableName + '"]');
    if (!card) return null;
    const rect = card.getBoundingClientRect();
    const root = document.getElementById('diagram').getBoundingClientRect();
    return {
      x: rect.left + rect.width / 2 - root.left,
      y: rect.top + rect.height / 2 - root.top,
      top: rect.top - root.top,
      bottom: rect.bottom - root.top,
      left: rect.left - root.left,
      right: rect.right - root.left,
    };
  }

  function drawRelations() {
    while (svg.firstChild) svg.removeChild(svg.firstChild);
    const diagram = document.getElementById('diagram');
    const w = diagram.clientWidth;
    const h = diagram.clientHeight;
    svg.setAttribute('width', w);
    svg.setAttribute('height', h);
    svg.setAttribute('viewBox', '0 0 ' + w + ' ' + h);

    const defs = document.createElementNS('http://www.w3.org/2000/svg', 'defs');
    const marker = document.createElementNS('http://www.w3.org/2000/svg', 'marker');
    marker.setAttribute('id', 'arrowhead');
    marker.setAttribute('markerWidth', '8');
    marker.setAttribute('markerHeight', '8');
    marker.setAttribute('refX', '6');
    marker.setAttribute('refY', '3');
    marker.setAttribute('orient', 'auto');
    const poly = document.createElementNS('http://www.w3.org/2000/svg', 'polygon');
    poly.setAttribute('points', '0 0, 8 3, 0 6');
    poly.setAttribute('fill', 'var(--line)');
    marker.appendChild(poly);
    defs.appendChild(marker);
    svg.appendChild(defs);

    const drawn = new Set();
    schema.tables.forEach(function (table) {
      table.foreign_keys.forEach(function (fk) {
        const key = table.name + '->' + fk.referenced_table + ':' + fk.columns.join(',');
        if (drawn.has(key)) return;
        drawn.add(key);
        const from = cardCenter(table.name);
        const to = cardCenter(fk.referenced_table);
        if (!from || !to) return;
        const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
        line.setAttribute('x1', from.x);
        line.setAttribute('y1', from.bottom);
        line.setAttribute('x2', to.x);
        line.setAttribute('y2', to.top);
        line.setAttribute('stroke', 'var(--line)');
        line.setAttribute('stroke-width', '1.5');
        line.setAttribute('marker-end', 'url(#arrowhead)');
        svg.appendChild(line);
      });
    });
  }

  drawRelations();
  window.addEventListener('resize', drawRelations);
})();
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema_design::model::{
        AbstractDataType, ColumnDesign, ForeignKeyDesign, PrimaryKeyDesign, SchemaDesign,
        TableDesign,
    };

    fn sample_schema() -> SchemaDesign {
        SchemaDesign {
            tables: vec![
                TableDesign {
                    name: "authors".to_string(),
                    schema: None,
                    columns: vec![ColumnDesign {
                        name: "id".to_string(),
                        abstract_type: AbstractDataType::Integer,
                        raw_type: "int".to_string(),
                        nullable: false,
                        default: None,
                        primary_key: true,
                        unique: false,
                        foreign_key: false,
                    }],
                    primary_key: PrimaryKeyDesign {
                        columns: vec!["id".to_string()],
                        autoincrement: true,
                    },
                    indexes: vec![],
                    foreign_keys: vec![],
                    constraints: vec![],
                },
                TableDesign {
                    name: "posts".to_string(),
                    schema: None,
                    columns: vec![
                        ColumnDesign {
                            name: "id".to_string(),
                            abstract_type: AbstractDataType::Integer,
                            raw_type: "int".to_string(),
                            nullable: false,
                            default: None,
                            primary_key: true,
                            unique: false,
                            foreign_key: false,
                        },
                        ColumnDesign {
                            name: "author_id".to_string(),
                            abstract_type: AbstractDataType::Integer,
                            raw_type: "int".to_string(),
                            nullable: false,
                            default: None,
                            primary_key: false,
                            unique: false,
                            foreign_key: true,
                        },
                    ],
                    primary_key: PrimaryKeyDesign {
                        columns: vec!["id".to_string()],
                        autoincrement: true,
                    },
                    indexes: vec![],
                    foreign_keys: vec![ForeignKeyDesign {
                        name: None,
                        columns: vec!["author_id".to_string()],
                        referenced_table: "authors".to_string(),
                        referenced_schema: None,
                        referenced_columns: vec!["id".to_string()],
                        on_delete: Some("NO ACTION".to_string()),
                        on_update: Some("NO ACTION".to_string()),
                    }],
                    constraints: vec![],
                },
            ],
        }
    }

    #[test]
    fn export_html_embeds_schema_json_and_badges() {
        let html = export_html(&sample_schema()).unwrap();
        assert!(html.contains("id=\"doido-schema-design\""));
        assert!(html.contains("\"authors\""));
        assert!(html.contains("class=\"badge pk\""));
        assert!(html.contains("class=\"badge fk\""));
        assert!(html.contains("data-tooltip"));
    }
}
