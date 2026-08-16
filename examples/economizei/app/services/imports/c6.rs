// C6 Bank CSV exports follow the same column layouts as Nubank for checking and credit card.
pub use super::nubank::{parse_checking, parse_credit_card};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::enums::Operation;

    #[test]
    fn parses_c6_checking_portuguese_headers() {
        let csv = "\
Data,Descricao,Valor
15/03/2025,Transferencia recebida,\"R$ 1500,00\"
16/03/2025,Pagamento boleto,\"-R$ 250,00\"
";
        let rows = parse_checking(csv).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].operation, Operation::Entrada);
        assert_eq!(rows[1].operation, Operation::Saida);
    }

    #[test]
    fn parses_c6_credit_card_portuguese_headers() {
        let csv = "\
Data,Estabelecimento,Categoria,Valor
10/03/2025,Supermercado Extra,Alimentacao,R$ 350,00
12/03/2025,Uber Trip,Transporte,47.50
";
        let rows = parse_credit_card(csv).unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].category_name.as_deref(), Some("Alimentacao"));
    }
}
