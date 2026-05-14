-- 在当前数据库中创建 token schema，并把当前数据库的默认搜索路径指向 token。
-- 注意：数据库本身由 API 启动前自动创建；sqlx migration 只能在目标库已存在后执行。
CREATE SCHEMA IF NOT EXISTS token;
DO $$
BEGIN
    EXECUTE format(
        'ALTER DATABASE %I SET search_path TO token, public',
        current_database()
    );
END
$$;
