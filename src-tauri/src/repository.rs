use crate::{
    access::{self, Snapshot},
    error::{AppError, Result},
    model::{FileRecord, Grant, Tag},
};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

pub struct Repository {
    pub(crate) conn: Connection,
}
fn positive(id: i64) -> Result<()> {
    if id > 0 {
        Ok(())
    } else {
        Err(AppError::invalid())
    }
}
fn tag_name(name: &str) -> Result<(String, String)> {
    let s = name.trim();
    if s.is_empty() || s.chars().count() > 64 || s.chars().any(char::is_control) {
        return Err(AppError::new(
            "INVALID",
            "标签需要 1–64 个字符，且不能包含控制字符。",
        ));
    }
    Ok((s.into(), s.to_lowercase()))
}
impl Repository {
    pub fn open(path: &Path) -> Result<Self> {
        let mut conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_secs(5))?;
        conn.execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;",
        )?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version > 1 {
            return Err(AppError::new(
                "SCHEMA",
                "数据库由较新版本创建，请使用兼容版本打开。",
            ));
        }
        if version == 0 {
            let tx = conn.transaction()?;
            tx.execute_batch(include_str!("../migrations/001_init.sql"))?;
            tx.commit()?;
        }
        let check: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if check != "ok" {
            return Err(AppError::new(
                "DATABASE",
                "数据库完整性检查失败，请保留数据库并恢复可靠备份。",
            ));
        }
        Ok(Self { conn })
    }
    pub fn register(&mut self, snapshot: &Snapshot) -> Result<i64> {
        let tx = self.conn.transaction()?;
        // A replacement at the same pathname must never inherit existing labels.
        let occupant: Option<String> = tx
            .query_row(
                "SELECT identity FROM file_records WHERE path_key=?",
                [access::path_key(&snapshot.path)],
                |r| r.get(0),
            )
            .optional()?;
        if occupant.is_some_and(|v| v != snapshot.identity) {
            return Err(AppError::new(
                "CONFLICT",
                "该路径已有其他文件的索引。请先处理旧索引，避免标签被错误继承。",
            ));
        }
        tx.execute("INSERT INTO permission_grants(kind,path,path_key,identity,created_at) VALUES('FILE',?1,?2,?3,?4) ON CONFLICT(kind,path_key,identity) DO UPDATE SET active=1,path=excluded.path", params![snapshot.path,access::path_key(&snapshot.path),snapshot.identity,access::now()])?;
        tx.execute("INSERT INTO file_records(identity,path,path_key,name,extension,size,modified_at,registered_at,status) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,'READY') ON CONFLICT(identity) DO UPDATE SET path=excluded.path,path_key=excluded.path_key,name=excluded.name,extension=excluded.extension,size=excluded.size,modified_at=excluded.modified_at,status='READY'", params![snapshot.identity,snapshot.path,access::path_key(&snapshot.path),snapshot.name,snapshot.extension,snapshot.size,snapshot.modified_at,access::now()])?;
        let id = tx.query_row(
            "SELECT id FROM file_records WHERE identity=?",
            [&snapshot.identity],
            |r| r.get(0),
        )?;
        tx.commit()?;
        Ok(id)
    }
    pub fn grant_directory(&mut self, snapshot: &Snapshot) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("INSERT INTO permission_grants(kind,path,path_key,identity,created_at) VALUES('DIRECTORY',?1,?2,?3,?4) ON CONFLICT(kind,path_key,identity) DO UPDATE SET active=1,path=excluded.path", params![snapshot.path,access::path_key(&snapshot.path),snapshot.identity,access::now()])?;
        tx.commit()?;
        Ok(())
    }
    pub fn grants(&self) -> Result<Vec<Grant>> {
        let mut q = self.conn.prepare("SELECT id,kind,path,identity,active FROM permission_grants ORDER BY active DESC,id DESC")?;
        let rows = q.query_map([], |r| {
            Ok(Grant {
                id: r.get(0)?,
                kind: r.get(1)?,
                path: r.get(2)?,
                identity: r.get(3)?,
                active: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
    pub fn revoke(&mut self, id: i64) -> Result<()> {
        positive(id)?;
        let changed = self
            .conn
            .execute("UPDATE permission_grants SET active=0 WHERE id=?", [id])?;
        if changed == 0 {
            return Err(AppError::invalid());
        }
        Ok(())
    }
    pub fn tags(&self) -> Result<Vec<Tag>> {
        let mut q = self
            .conn
            .prepare("SELECT id,name FROM tags ORDER BY name_key")?;
        let rows = q.query_map([], |r| {
            Ok(Tag {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
    pub fn create_tag(&self, name: &str) -> Result<()> {
        let (name, key) = tag_name(name)?;
        self.conn.execute(
            "INSERT INTO tags(name,name_key) VALUES(?,?)",
            params![name, key],
        )?;
        Ok(())
    }
    pub fn rename_tag(&self, id: i64, name: &str) -> Result<()> {
        positive(id)?;
        let (name, key) = tag_name(name)?;
        if self.conn.execute(
            "UPDATE tags SET name=?,name_key=? WHERE id=?",
            params![name, key, id],
        )? == 0
        {
            return Err(AppError::invalid());
        }
        Ok(())
    }
    pub fn set_tag(&self, file_id: i64, tag_id: i64, attached: bool) -> Result<()> {
        positive(file_id)?;
        positive(tag_id)?;
        if attached {
            self.conn.execute("INSERT INTO file_tags(file_id,tag_id,source) VALUES(?,?,'USER') ON CONFLICT DO NOTHING", params![file_id,tag_id])?;
        } else {
            self.conn.execute(
                "DELETE FROM file_tags WHERE file_id=? AND tag_id=?",
                params![file_id, tag_id],
            )?;
        }
        Ok(())
    }
    fn row_file(r: &rusqlite::Row<'_>) -> rusqlite::Result<FileRecord> {
        Ok(FileRecord {
            id: r.get(0)?,
            identity: r.get(1)?,
            path: r.get(2)?,
            name: r.get(3)?,
            extension: r.get(4)?,
            size: r.get(5)?,
            modified_at: r.get(6)?,
            registered_at: r.get(7)?,
            status: r.get(8)?,
            tags: vec![],
        })
    }
    pub fn file(&self, id: i64) -> Result<FileRecord> {
        positive(id)?;
        let mut f = self.conn.query_row("SELECT id,identity,path,name,extension,size,modified_at,registered_at,status FROM file_records WHERE id=?", [id], Self::row_file)?;
        f.tags = self.file_tags(id)?;
        Ok(f)
    }
    fn file_tags(&self, id: i64) -> Result<Vec<Tag>> {
        let mut q = self.conn.prepare("SELECT t.id,t.name FROM tags t JOIN file_tags ft ON ft.tag_id=t.id WHERE ft.file_id=? ORDER BY t.name_key")?;
        let rows = q.query_map([id], |r| {
            Ok(Tag {
                id: r.get(0)?,
                name: r.get(1)?,
            })
        })?;
        Ok(rows.collect::<std::result::Result<Vec<_>, _>>()?)
    }
    pub fn search(
        &self,
        query: &str,
        tag_id: Option<i64>,
        extension: &str,
    ) -> Result<Vec<FileRecord>> {
        if query.chars().count() > 256 || extension.len() > 64 {
            return Err(AppError::invalid());
        }
        if let Some(id) = tag_id {
            positive(id)?;
        }
        let mut q = self.conn.prepare("SELECT id,identity,path,name,extension,size,modified_at,registered_at,status FROM file_records WHERE instr(lower(name),lower(?1))>0 AND (?2 IS NULL OR EXISTS(SELECT 1 FROM file_tags WHERE file_id=file_records.id AND tag_id=?2)) AND (?3='' OR extension=?3) ORDER BY registered_at DESC,id DESC")?;
        let mut files = q
            .query_map(
                params![query, tag_id, extension.to_lowercase()],
                Self::row_file,
            )?
            .collect::<std::result::Result<Vec<_>, _>>()?;
        for file in &mut files {
            file.tags = self.file_tags(file.id)?;
        }
        Ok(files)
    }
    pub fn refresh(&mut self, id: i64) -> Result<FileRecord> {
        // Serialize authorization check and read against revocations in other instances.
        let tx = rusqlite::Transaction::new_unchecked(
            &self.conn,
            rusqlite::TransactionBehavior::Immediate,
        )?;
        let old = self.file(id)?;
        let access_result =
            access::authorized_snapshot(Path::new(&old.path), &old.identity, &self.grants()?);
        match &access_result {
            Ok(s) => {
                self.conn.execute(
                    "UPDATE file_records SET size=?,modified_at=?,status='READY' WHERE id=?",
                    params![s.size, s.modified_at, id],
                )?;
            }
            Err(e) => {
                let status = match e.code {
                    "MISSING" => "MISSING",
                    "UNAUTHORIZED" => "UNAUTHORIZED",
                    "CHANGED" => "CHANGED",
                    _ => "ERROR",
                };
                self.conn.execute(
                    "UPDATE file_records SET status=? WHERE id=?",
                    params![status, id],
                )?;
                if status == "ERROR" {
                    tx.commit()?;
                    return Err(e.clone());
                }
            }
        }
        let result = self.file(id)?;
        tx.commit()?;
        Ok(result)
    }
    pub fn relink(&mut self, id: i64, snapshot: &Snapshot) -> Result<()> {
        let old = self.file(id)?;
        if old.identity != snapshot.identity {
            return Err(AppError::new(
                "IDENTITY",
                "所选文件与原记录身份不一致。本版仅支持同卷移动后的原文件，不接受副本或同名文件。",
            ));
        }
        self.register(snapshot)?;
        Ok(())
    }
    pub fn delete_index(&self, id: i64) -> Result<()> {
        positive(id)?;
        if self
            .conn
            .execute("DELETE FROM file_records WHERE id=?", [id])?
            == 0
        {
            return Err(AppError::invalid());
        }
        Ok(())
    }
}
