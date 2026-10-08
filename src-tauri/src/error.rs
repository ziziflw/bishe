use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub code: &'static str,
    pub message: String,
}
pub type Result<T> = std::result::Result<T, AppError>;
impl AppError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
    pub fn invalid() -> Self {
        Self::new("INVALID", "输入无效，请检查后重试。")
    }
    pub fn denied() -> Self {
        Self::new(
            "UNAUTHORIZED",
            "没有有效读取授权，请重新选择文件或授权目录。",
        )
    }
}
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for AppError {}
impl From<rusqlite::Error> for AppError {
    fn from(_: rusqlite::Error) -> Self {
        Self::new(
            "DATABASE",
            "数据库操作失败，可能存在重复名称、数据冲突、只读或磁盘空间不足；本次事务已回滚。",
        )
    }
}
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::NotFound => Self::new("MISSING", "原文件或所在目录不存在。"),
            std::io::ErrorKind::PermissionDenied => {
                Self::new("ACCESS", "文件无法读取：权限不足或被其他程序占用。")
            }
            _ => Self::new("IO", "无法安全读取文件，可能被占用或路径不可用。"),
        }
    }
}
