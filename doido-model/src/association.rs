//! Declarative association conventions (Rails `belongs_to`/`has_many`/…).
//!
//! sea-orm already models relations at the type level; this adds the Rails
//! naming conventions (foreign keys, target tables, HABTM join tables) that
//! generators and query helpers reuse, derived from [`doido_core::Inflector`].

use doido_core::Inflector;

/// The kind of association between two models.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssociationKind {
    BelongsTo,
    HasOne,
    HasMany,
    HasAndBelongsToMany,
}

/// A resolved association: its name plus the conventional foreign key and table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Association {
    pub name: String,
    pub kind: AssociationKind,
    pub foreign_key: String,
    pub table: String,
}

impl Association {
    /// `belongs_to :author` — this table holds `author_id`; target is `authors`.
    pub fn belongs_to(name: &str) -> Self {
        Self {
            name: name.to_string(),
            kind: AssociationKind::BelongsTo,
            foreign_key: Inflector::foreign_key(name),
            table: Inflector::tableize(name),
        }
    }

    /// `has_one :profile` on `owner` — child table holds `{owner}_id`.
    pub fn has_one(owner: &str, name: &str) -> Self {
        Self {
            name: name.to_string(),
            kind: AssociationKind::HasOne,
            foreign_key: Inflector::foreign_key(owner),
            table: Inflector::tableize(name),
        }
    }

    /// `has_many :comments` on `owner` — child table holds `{owner}_id`. The
    /// association name is already plural, so it is singularized before tableizing.
    pub fn has_many(owner: &str, name: &str) -> Self {
        Self {
            name: name.to_string(),
            kind: AssociationKind::HasMany,
            foreign_key: Inflector::foreign_key(owner),
            table: Inflector::tableize(&Inflector::singularize(name)),
        }
    }

    /// `has_and_belongs_to_many :tags` — target table `tags`, join FK `tag_id`.
    pub fn has_and_belongs_to_many(name: &str) -> Self {
        Self {
            name: name.to_string(),
            kind: AssociationKind::HasAndBelongsToMany,
            foreign_key: Inflector::foreign_key(&Inflector::singularize(name)),
            table: Inflector::tableize(&Inflector::singularize(name)),
        }
    }
}

/// The conventional HABTM join-table name: the two tables, sorted, joined by `_`
/// (e.g. `("Article", "Tag") → "articles_tags"`).
pub fn join_table(a: &str, b: &str) -> String {
    let mut tables = [Inflector::tableize(a), Inflector::tableize(b)];
    tables.sort();
    format!("{}_{}", tables[0], tables[1])
}

/// A polymorphic `belongs_to` (Rails `belongs_to :commentable, polymorphic: true`),
/// which stores the target's class name in `{name}_type` and its id in `{name}_id`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolymorphicAssociation {
    pub name: String,
    pub type_column: String,
    pub id_column: String,
}

impl PolymorphicAssociation {
    pub fn belongs_to(name: &str) -> Self {
        Self {
            name: name.to_string(),
            type_column: format!("{name}_type"),
            id_column: format!("{name}_id"),
        }
    }
}

/// The default single-table-inheritance discriminator column (Rails `type`).
pub fn sti_type_column() -> &'static str {
    "type"
}
