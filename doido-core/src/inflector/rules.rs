use super::inflections::Inflections;

pub fn defaults() -> Inflections {
    let mut i = Inflections::new();

    // ── Plural rules ──────────────────────────────────────────────────────────
    // Added lowest-priority first; last-added rule is tried first.

    i.plural(r"$", "s"); // catch-all: word → words
    i.plural(r"(s|x|z|ch|sh)$", "${1}es"); // box→boxes, watch→watches
    i.plural(r"([^aeiouy])y$", "${1}ies"); // city→cities  (vowel+y stays: day→days via catch-all)
    i.plural(r"(tomat|potat)o$", "${1}oes"); // tomato→tomatoes
    i.plural(r"sis$", "ses"); // analysis→analyses
    i.plural(r"([ti])um$", "${1}a"); // datum→data, medium→media
    i.plural(r"(quiz)$", "${1}zes"); // quiz→quizzes

    // ── Singular rules ────────────────────────────────────────────────────────
    // Added lowest-priority first.

    i.singular(r"s$", ""); // catch-all: dogs→dog
    i.singular(r"(ss|us|is)$", "${1}"); // class→class, radius→radius, analysis→analysis
    i.singular(r"(x|ch|ss|sh)es$", "${1}"); // boxes→box, watches→watch
    i.singular(r"([^aeiouy])ies$", "${1}y"); // cities→city
    i.singular(r"(tomat|potat)oes$", "${1}o"); // potatoes→potato
    i.singular(r"ses$", "sis"); // analyses→analysis
    i.singular(r"([ti])a$", "${1}um"); // data→datum

    // ── Default irregulars ────────────────────────────────────────────────────

    i.irregular("person", "people");
    i.irregular("man", "men");
    i.irregular("child", "children");
    i.irregular("move", "moves");
    i.irregular("zombie", "zombies");

    // ── Default uncountables ──────────────────────────────────────────────────

    for word in &[
        "equipment",
        "information",
        "rice",
        "money",
        "species",
        "series",
        "fish",
        "sheep",
        "jeans",
        "police",
    ] {
        i.uncountable(word);
    }

    i
}
