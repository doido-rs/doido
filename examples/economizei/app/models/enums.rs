use doido::model::sea_orm as sea_orm;
use doido::model::sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
    Serialize,
    Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum MembershipRole {
    #[sea_orm(string_value = "owner")]
    #[serde(rename = "owner")]
    Owner,
    #[sea_orm(string_value = "admin")]
    #[serde(rename = "admin")]
    Admin,
    #[sea_orm(string_value = "member")]
    #[serde(rename = "member")]
    Member,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
    Serialize,
    Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum AccountType {
    #[sea_orm(string_value = "corrente")]
    #[serde(rename = "corrente")]
    Corrente,
    #[sea_orm(string_value = "investimento")]
    #[serde(rename = "investimento")]
    Investimento,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
    Serialize,
    Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum Operation {
    #[sea_orm(string_value = "ENTRADA")]
    #[serde(rename = "ENTRADA")]
    Entrada,
    #[sea_orm(string_value = "SAIDA")]
    #[serde(rename = "SAIDA")]
    Saida,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
    Serialize,
    Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum MovementType {
    #[sea_orm(string_value = "balance")]
    #[serde(rename = "balance")]
    Balance,
    #[sea_orm(string_value = "credit_card")]
    #[serde(rename = "credit_card")]
    CreditCard,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
    Serialize,
    Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum ImportSource {
    #[sea_orm(string_value = "nubank")]
    #[serde(rename = "nubank")]
    Nubank,
    #[sea_orm(string_value = "c6")]
    #[serde(rename = "c6")]
    C6,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
    Serialize,
    Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum ImportStatementType {
    #[sea_orm(string_value = "checking_account")]
    #[serde(rename = "checking_account")]
    CheckingAccount,
    #[sea_orm(string_value = "credit_card")]
    #[serde(rename = "credit_card")]
    CreditCard,
}

#[derive(
    Clone,
    Debug,
    PartialEq,
    Eq,
    EnumIter,
    DeriveActiveEnum,
    Serialize,
    Deserialize,
)]
#[sea_orm(rs_type = "String", db_type = "Text")]
pub enum ImportStatus {
    #[sea_orm(string_value = "completed")]
    #[serde(rename = "completed")]
    Completed,
    #[sea_orm(string_value = "failed")]
    #[serde(rename = "failed")]
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_type_deserializes_form_string_values() {
        let corrente: AccountType =
            serde_json::from_str("\"corrente\"").expect("corrente");
        assert_eq!(corrente, AccountType::Corrente);

        let investimento: AccountType =
            serde_json::from_str("\"investimento\"").expect("investimento");
        assert_eq!(investimento, AccountType::Investimento);
    }

    #[test]
    fn operation_deserializes_form_string_values() {
        let entrada: Operation =
            serde_json::from_str("\"ENTRADA\"").expect("ENTRADA");
        assert_eq!(entrada, Operation::Entrada);

        let saida: Operation = serde_json::from_str("\"SAIDA\"").expect("SAIDA");
        assert_eq!(saida, Operation::Saida);
    }

    #[test]
    fn movement_type_deserializes_form_string_values() {
        let balance: MovementType =
            serde_json::from_str("\"balance\"").expect("balance");
        assert_eq!(balance, MovementType::Balance);

        let credit_card: MovementType =
            serde_json::from_str("\"credit_card\"").expect("credit_card");
        assert_eq!(credit_card, MovementType::CreditCard);
    }
}
