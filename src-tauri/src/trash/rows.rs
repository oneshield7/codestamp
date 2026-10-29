use std::collections::BTreeSet;

use rusqlite::types::Value as Cell;
use rusqlite::{Connection, ToSql, params_from_iter};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// The tables a deleted collection or request keeps rows from. Restoring reads the table names
/// back through this list, so a saved copy can never name any other table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Table {
    Collections,
    Sections,
    Requests,
    RequestExamples,
    Environments,
    Variables,
    CatalogInfo,
    CollectionRuns,
    TeamLinks,
    TeamAccessGroups,
    TeamAccessRequests,
    AppSecrets,
}

impl Table {
    const fn name(self) -> &'static str {
        match self {
            Self::Collections => "collections",
            Self::Sections => "sections",
            Self::Requests => "requests",
            Self::RequestExamples => "request_examples",
            Self::Environments => "environments",
            Self::Variables => "variables",
            Self::CatalogInfo => "catalog_info",
            Self::CollectionRuns => "collection_runs",
            Self::TeamLinks => "team_links",
            Self::TeamAccessGroups => "team_access_groups",
            Self::TeamAccessRequests => "team_access_requests",
            Self::AppSecrets => "app_secrets",
        }
    }
}

/// References that SQLite sets to NULL when their row is deleted, put back on restore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkKind {
    /// An Orchestration step and the request it runs.
    FlowStepRequest,
    HistoryRequest,
    HistoryCollection,
}

impl LinkKind {
    /// Only a link that is still empty is restored, so a step pointed elsewhere since stays so.
    const fn restore_sql(self) -> &'static str {
        match self {
            Self::FlowStepRequest => {
                "UPDATE flow_steps SET request_id = ?1 WHERE id = ?2 AND request_id IS NULL"
            }
            Self::HistoryRequest => {
                "UPDATE history SET request_id = ?1 WHERE id = ?2 AND request_id IS NULL"
            }
            Self::HistoryCollection => {
                "UPDATE history SET collection_id = ?1 WHERE id = ?2 AND collection_id IS NULL"
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TableRows {
    pub table: Table,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Stored>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    pub kind: LinkKind,
    /// The row that pointed at the deleted item.
    pub row: String,
    /// The id it pointed at.
    pub target: String,
}

/// Everything a deleted item was, as rows in the order they are inserted again.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Saved {
    pub tables: Vec<TableRows>,
    pub links: Vec<Link>,
}

/// One SQLite value. Blobs are the vault-encrypted secrets and stay encrypted here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Stored {
    Null,
    Integer(i64),
    Real(f64),
    Text(String),
    Blob(Vec<u8>),
}

impl From<Cell> for Stored {
    fn from(cell: Cell) -> Self {
        match cell {
            Cell::Null => Self::Null,
            Cell::Integer(value) => Self::Integer(value),
            Cell::Real(value) => Self::Real(value),
            Cell::Text(value) => Self::Text(value),
            Cell::Blob(value) => Self::Blob(value),
        }
    }
}

impl From<Stored> for Cell {
    fn from(stored: Stored) -> Self {
        match stored {
            Stored::Null => Self::Null,
            Stored::Integer(value) => Self::Integer(value),
            Stored::Real(value) => Self::Real(value),
            Stored::Text(value) => Self::Text(value),
            Stored::Blob(value) => Self::Blob(value),
        }
    }
}

impl Saved {
    /// Copies the rows of `table` that `filter` (a fixed WHERE clause from this module's callers)
    /// selects.
    pub fn keep(
        &mut self,
        conn: &Connection,
        table: Table,
        filter: &str,
        params: &[&dyn ToSql],
    ) -> Result<(), AppError> {
        let mut statement =
            conn.prepare(&format!("SELECT * FROM {} WHERE {filter}", table.name()))?;
        let columns: Vec<String> = statement
            .column_names()
            .into_iter()
            .map(str::to_owned)
            .collect();
        let count = columns.len();
        let rows = statement
            .query_map(params, |row| {
                (0..count)
                    .map(|index| row.get::<_, Cell>(index).map(Stored::from))
                    .collect::<Result<Vec<_>, _>>()
            })?
            .collect::<Result<Vec<_>, _>>()?;
        if !rows.is_empty() {
            self.tables.push(TableRows {
                table,
                columns,
                rows,
            });
        }
        Ok(())
    }

    /// Remembers which rows of a fixed query (`SELECT id, target …`) point at the deleted item.
    pub fn keep_links(
        &mut self,
        conn: &Connection,
        kind: LinkKind,
        query: &str,
        params: &[&dyn ToSql],
    ) -> Result<(), AppError> {
        let mut statement = conn.prepare(query)?;
        let links = statement
            .query_map(params, |row| {
                Ok(Link {
                    kind,
                    row: row.get(0)?,
                    target: row.get(1)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        self.links.extend(links);
        Ok(())
    }

    /// Changes one column in every row of `table`, such as the collection a request goes back to.
    pub fn set(&mut self, table: Table, column: &str, value: &Stored) {
        for rows in self.tables.iter_mut().filter(|rows| rows.table == table) {
            if let Some(index) = rows.columns.iter().position(|name| name == column) {
                for row in &mut rows.rows {
                    if let Some(cell) = row.get_mut(index) {
                        cell.clone_from(value);
                    }
                }
            }
        }
    }

    /// Inserts every row again, then the links. The caller wraps it in a transaction.
    pub fn restore(self, conn: &Connection) -> Result<(), AppError> {
        for rows in self.tables {
            let known = columns_of(conn, rows.table)?;
            if rows.columns.is_empty() || rows.columns.iter().any(|column| !known.contains(column))
            {
                return Err(AppError::invalid(
                    "This item was saved by a newer Codestamp, so it cannot be restored here",
                ));
            }
            let names: Vec<String> = rows
                .columns
                .iter()
                .map(|column| format!("\"{column}\""))
                .collect();
            let marks = vec!["?"; rows.columns.len()].join(", ");
            let sql = format!(
                "INSERT INTO {} ({}) VALUES ({marks})",
                rows.table.name(),
                names.join(", ")
            );
            let mut statement = conn.prepare(&sql)?;
            for row in rows.rows {
                if row.len() != rows.columns.len() {
                    return Err(AppError::Storage);
                }
                statement.execute(params_from_iter(row.into_iter().map(Cell::from)))?;
            }
        }
        for link in self.links {
            conn.execute(link.kind.restore_sql(), [&link.target, &link.row])?;
        }
        Ok(())
    }
}

/// The table's real columns, the only names an INSERT may use.
fn columns_of(conn: &Connection, table: Table) -> Result<BTreeSet<String>, AppError> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({})", table.name()))?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<_, _>>()?;
    Ok(names)
}
