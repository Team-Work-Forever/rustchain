use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Transaction<TData> {
    pub from: String,
    pub to: String,
    pub data: TData,
}

impl<TData: Serialize> Transaction<TData> {
    pub fn new(from: String, to: String, data: TData) -> Transaction<TData> {
        Transaction { from, to, data }
    }

    pub fn to_json_string(&self) -> String {
        serde_json::to_string(self).expect("msg")
    }
}
