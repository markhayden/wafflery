use super::{Error, Result, db::Database};
use rusqlite::{OptionalExtension, params};

#[derive(Clone)]
pub struct BaselineStore(pub Database);
impl BaselineStore {
    pub async fn get(&self, tool: String, action: String) -> Result<Option<String>> {
        self.0
            .call(move |db| {
                Ok(db
                    .query_row(
                        "SELECT attempt_id FROM baselines WHERE tool=?1 AND action=?2",
                        params![tool, action],
                        |row| row.get(0),
                    )
                    .optional()?)
            })
            .await
    }
    pub async fn set(&self, tool: String, action: String, id: Option<String>) -> Result<()> {
        self.0.call(move |db| {
            let tx=db.transaction()?;
            if let Some(id)=id {
                let context:Option<(String,String,String)>=tx.query_row(
                    "SELECT json_extract(record,'$.request.tool'),json_extract(record,'$.request.action'),json_extract(record,'$.execution') FROM attempts WHERE id=?1",[&id],
                    |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
                let (saved_tool,saved_action,status)=context.ok_or(Error::Missing("Saved attempt not found"))?;
                if tool!=saved_tool || action!=saved_action || status!="completed" {
                    return Err(Error::Conflict("Choose a completed run from the same tool and action"));
                }
                tx.execute("INSERT INTO baselines(tool,action,attempt_id) VALUES(?1,?2,?3) ON CONFLICT(tool,action) DO UPDATE SET attempt_id=excluded.attempt_id",params![tool,action,id])?;
            } else {
                tx.execute("DELETE FROM baselines WHERE tool=?1 AND action=?2",params![tool,action])?;
            }
            tx.commit()?;Ok(())
        }).await
    }
}
