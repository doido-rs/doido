use crate::models::enums::Operation;
use crate::models::membership::Entity as MembershipEntity;
use crate::models::transaction::{
    Column as TransactionColumn, Entity as TransactionEntity,
};
use doido::model::sea_orm::entity::prelude::*;
use doido::model::sea_orm::DatabaseConnection;
use serde::Serialize;
use std::str::FromStr;

#[derive(Debug, Serialize)]
pub struct HealthReport {
    pub savings_rate: f64,
    pub expense_ratio: f64,
    pub total_income: String,
    pub total_expenses: String,
}

#[derive(Debug, Serialize)]
pub struct SpendingGoal {
    pub category_id: i64,
    pub category_name: String,
    pub target_amount: String,
    pub spent_amount: String,
    pub remaining_amount: String,
}

#[derive(Debug, Serialize)]
pub struct SpendingGoalsReport {
    pub savings_target_rate: f64,
    pub goals: Vec<SpendingGoal>,
}

#[derive(Debug, Serialize)]
pub struct ChartSlice {
    pub label: String,
    pub value: f64,
}

#[derive(Debug, Serialize)]
pub struct ReportsChartData {
    pub balance: Vec<ChartSlice>,
    pub expenses_by_category: Vec<ChartSlice>,
    pub budget_by_category: Vec<ChartSlice>,
    pub savings_split: Vec<ChartSlice>,
}

#[derive(Debug, Serialize)]
pub struct ReportsIndex {
    pub health: HealthReport,
    pub spending_goals: SpendingGoalsReport,
    pub charts: ReportsChartData,
}

pub async fn health_report(
    db: &DatabaseConnection,
    company_id: i64,
) -> doido::Result<HealthReport> {
    let transactions = TransactionEntity::find()
        .filter(TransactionColumn::CompanyId.eq(company_id))
        .all(db)
        .await?;

    let mut income = Decimal::ZERO;
    let mut expenses = Decimal::ZERO;

    for tx in transactions {
        match tx.operation {
            Operation::Entrada => income += tx.amount,
            Operation::Saida => expenses += tx.amount,
        }
    }

    let salary = total_salary(db, company_id).await?;
    let base = if salary > Decimal::ZERO {
        salary
    } else if income > Decimal::ZERO {
        income
    } else {
        Decimal::ONE
    };

    let savings = income - expenses;
    let savings_rate =
        (savings / base).to_string().parse::<f64>().unwrap_or(0.0);
    let expense_ratio =
        (expenses / base).to_string().parse::<f64>().unwrap_or(0.0);

    Ok(HealthReport {
        savings_rate,
        expense_ratio,
        total_income: income.to_string(),
        total_expenses: expenses.to_string(),
    })
}

pub async fn spending_goals_report(
    db: &DatabaseConnection,
    company_id: i64,
) -> doido::Result<SpendingGoalsReport> {
    use crate::models::category::{
        Column as CategoryColumn, Entity as CategoryEntity,
    };

    let categories = CategoryEntity::find()
        .filter(CategoryColumn::CompanyId.eq(company_id))
        .all(db)
        .await?;

    let salary = total_salary(db, company_id).await?;
    let spendable = salary * Decimal::from_str("0.80").unwrap_or(Decimal::ZERO);
    let per_category = if categories.is_empty() {
        Decimal::ZERO
    } else {
        spendable / Decimal::from(categories.len() as i64)
    };

    let mut goals = Vec::new();
    for category in categories {
        let spent = TransactionEntity::find()
            .filter(TransactionColumn::CompanyId.eq(company_id))
            .filter(TransactionColumn::CategoryId.eq(category.id))
            .filter(TransactionColumn::Operation.eq(Operation::Saida))
            .all(db)
            .await?
            .into_iter()
            .map(|tx| tx.amount)
            .fold(Decimal::ZERO, |acc, amount| acc + amount);

        let remaining = per_category - spent;
        goals.push(SpendingGoal {
            category_id: category.id,
            category_name: category.name,
            target_amount: per_category.to_string(),
            spent_amount: spent.to_string(),
            remaining_amount: remaining.to_string(),
        });
    }

    Ok(SpendingGoalsReport {
        savings_target_rate: 0.20,
        goals,
    })
}

pub async fn reports_index(
    db: &DatabaseConnection,
    company_id: i64,
) -> doido::Result<ReportsIndex> {
    let health = health_report(db, company_id).await?;
    let spending_goals = spending_goals_report(db, company_id).await?;
    let charts = build_chart_data(&health, &spending_goals, db, company_id).await?;
    Ok(ReportsIndex {
        health,
        spending_goals,
        charts,
    })
}

fn decimal_to_f64(value: Decimal) -> f64 {
    value.to_string().parse::<f64>().unwrap_or(0.0)
}

async fn build_chart_data(
    health: &HealthReport,
    spending_goals: &SpendingGoalsReport,
    db: &DatabaseConnection,
    company_id: i64,
) -> doido::Result<ReportsChartData> {
    use crate::services::i18n;

    let income = health
        .total_income
        .parse::<Decimal>()
        .unwrap_or(Decimal::ZERO);
    let expenses = health
        .total_expenses
        .parse::<Decimal>()
        .unwrap_or(Decimal::ZERO);
    let savings = (income - expenses).max(Decimal::ZERO);

    let balance = if income > Decimal::ZERO || expenses > Decimal::ZERO {
        vec![
            ChartSlice {
                label: i18n::t("reports.savings"),
                value: decimal_to_f64(savings),
            },
            ChartSlice {
                label: i18n::t("reports.expenses"),
                value: decimal_to_f64(expenses),
            },
        ]
    } else {
        vec![ChartSlice {
            label: i18n::t("reports.no_data"),
            value: 1.0,
        }]
    };

    let expenses_by_category: Vec<ChartSlice> = spending_goals
        .goals
        .iter()
        .filter_map(|goal| {
            let spent = goal.spent_amount.parse::<Decimal>().ok()?;
            if spent <= Decimal::ZERO {
                return None;
            }
            Some(ChartSlice {
                label: goal.category_name.clone(),
                value: decimal_to_f64(spent),
            })
        })
        .collect();

    let expenses_by_category = if expenses_by_category.is_empty() {
        vec![ChartSlice {
            label: i18n::t("reports.no_data"),
            value: 1.0,
        }]
    } else {
        expenses_by_category
    };

    let budget_by_category: Vec<ChartSlice> = spending_goals
        .goals
        .iter()
        .map(|goal| ChartSlice {
            label: goal.category_name.clone(),
            value: decimal_to_f64(
                goal.target_amount.parse::<Decimal>().unwrap_or(Decimal::ZERO),
            ),
        })
        .collect();

    let budget_by_category = if budget_by_category.is_empty() {
        vec![ChartSlice {
            label: i18n::t("reports.no_data"),
            value: 1.0,
        }]
    } else {
        budget_by_category
    };

    let salary = total_salary(db, company_id).await?;
    let savings_amount = salary * Decimal::from_str("0.20").unwrap_or(Decimal::ZERO);
    let spendable = salary * Decimal::from_str("0.80").unwrap_or(Decimal::ZERO);

    let savings_split = if salary > Decimal::ZERO {
        vec![
            ChartSlice {
                label: i18n::t("reports.savings_target"),
                value: decimal_to_f64(savings_amount),
            },
            ChartSlice {
                label: i18n::t("reports.spendable"),
                value: decimal_to_f64(spendable),
            },
        ]
    } else {
        vec![ChartSlice {
            label: i18n::t("reports.no_data"),
            value: 1.0,
        }]
    };

    Ok(ReportsChartData {
        balance,
        expenses_by_category,
        budget_by_category,
        savings_split,
    })
}

async fn total_salary(
    db: &DatabaseConnection,
    company_id: i64,
) -> doido::Result<Decimal> {
    use crate::models::membership::Column as MembershipColumn;

    let memberships = MembershipEntity::find()
        .filter(MembershipColumn::CompanyId.eq(company_id))
        .all(db)
        .await?;

    Ok(memberships
        .into_iter()
        .filter_map(|m| m.salary)
        .fold(Decimal::ZERO, |acc, salary| acc + salary))
}
