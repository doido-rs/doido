use doido::controller::{Context, Response};

pub fn escape_field(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

pub fn build_csv(headers: &[&str], rows: Vec<Vec<String>>) -> String {
    let mut lines = Vec::with_capacity(rows.len() + 1);
    lines.push(
        headers
            .iter()
            .map(|header| escape_field(header))
            .collect::<Vec<_>>()
            .join(","),
    );

    for row in rows {
        lines.push(
            row.iter()
                .map(|cell| escape_field(cell))
                .collect::<Vec<_>>()
                .join(","),
        );
    }

    lines.join("\n")
}

pub fn attachment(ctx: &Context, filename: &str, body: String) -> Response {
    ctx.send_data(body.into_bytes(), "text/csv; charset=utf-8", Some(filename))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_escapes_commas() {
        assert_eq!(escape_field("a,b"), "\"a,b\"");
    }
}
