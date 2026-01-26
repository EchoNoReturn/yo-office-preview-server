use moka::future::Cache;
use once_cell::sync::Lazy;
use std::time::Duration;

static KV_CACHE: Lazy<Cache<String, String>> = Lazy::new(|| {
    Cache::builder()
        .time_to_live(Duration::from_secs(60 * 60)) // 1 小时
        .max_capacity(10_000) // 可选：最大条目数
        .build()
});

/**
 * 设置缓存
 */
pub async fn set_cache(key: String, value: String) {
    KV_CACHE.insert(key, value).await;
}

/**
 * 获取缓存
 */
pub async fn get_cache(key: &str) -> Option<String> {
    KV_CACHE.get(key).await
}
