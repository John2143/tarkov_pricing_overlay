#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub uid: String,
    pub name: String,
    pub short_name: String,
    pub price: i64,
    pub base_price: i64,
    #[serde(rename = "avg24hPrice")]
    pub avg24h_price: i64,
    #[serde(rename = "avg7daysPrice")]
    pub avg7d_price: i64,
    pub trader_name: String,
    pub trader_price: i64,
    pub trader_price_cur: String,
    pub updated: String,
    pub slots: i64,
    #[serde(rename = "diff24h")]
    pub diff24h: f64,
    #[serde(rename = "diff7days")]
    pub diff7d: f64,
    //pub icon: String,
    //pub link: String,
    //pub wiki_link: String,
    //pub img: String,
    //pub img_big: String,
    //pub bsg_id: String,
    //pub is_functional: bool,
    //pub reference: String,
}

