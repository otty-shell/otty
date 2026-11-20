-- The made-up OpenCode database of the fixtures, in the second shape
-- (OpenCode 2): two sessions, a helper session, their messages and a
-- DeepSeek key in the `credential` table. Every value is made up.
-- Rebuild it with:
--   rm -f fixtures/.local/share/opencode/opencode.db
--   sqlite3 fixtures/.local/share/opencode/opencode.db < fixture-database.sql
CREATE TABLE session_v2 (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    parent_id TEXT,
    directory TEXT NOT NULL,
    title TEXT,
    version TEXT NOT NULL,
    time_created INTEGER NOT NULL,
    time_updated INTEGER NOT NULL
);
CREATE TABLE session_message (
    id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    type TEXT NOT NULL,
    seq INTEGER NOT NULL,
    time_created INTEGER NOT NULL,
    time_updated INTEGER NOT NULL,
    data TEXT NOT NULL
);
CREATE TABLE credential (
    id TEXT PRIMARY KEY,
    integration_id TEXT,
    label TEXT NOT NULL,
    value TEXT NOT NULL
);

INSERT INTO session_v2 VALUES
    ('ses_shop', 'prj_shop', NULL, '/home/dev/shop', 'Fix the cart', '2.3.1',
     1794816000000, 1794819600000),
    ('ses_shop_helper', 'prj_shop', 'ses_shop', '/home/dev/shop', 'Explore',
     '2.3.1', 1794819600000, 1794819700000),
    ('ses_blog', 'prj_blog', NULL, '/home/dev/blog', 'New post', '2.3.2',
     1791367200000, 1794823200000);

INSERT INTO session_message VALUES
    ('msg_shop_1', 'ses_shop', 'user', 1, 1794816000000, 1794816000000,
     '{"agent":"build","model":{"id":"gpt-5.5","providerID":"openai"}}'),
    ('msg_shop_2', 'ses_shop', 'assistant', 2, 1794819600000, 1794819600000,
     '{"agent":"build","model":{"id":"gpt-5.5","providerID":"openai"},"content":[],"finish":"stop","cost":0.012,"tokens":{"input":1000,"output":50,"reasoning":20,"cache":{"read":3000,"write":0}},"time":{"created":1794819600000,"completed":1794819605000}}'),
    ('msg_helper_1', 'ses_shop_helper', 'user', 3, 1794819650000, 1794819650000,
     '{"agent":"explore","model":{"id":"gpt-5.5","providerID":"openai"}}'),
    ('msg_helper_2', 'ses_shop_helper', 'assistant', 4, 1794819700000, 1794819700000,
     '{"agent":"explore","model":{"id":"gpt-5.5","providerID":"openai"},"content":[],"finish":"stop","cost":0.002,"tokens":{"input":200,"output":10,"reasoning":0,"cache":{"read":0,"write":10}},"time":{"created":1794819700000,"completed":1794819702000}}'),
    ('msg_blog_1', 'ses_blog', 'assistant', 5, 1791367200000, 1791367200000,
     '{"agent":"build","model":{"id":"deepseek-chat","providerID":"deepseek"},"content":[],"finish":"stop","cost":0.001,"tokens":{"input":80,"output":8,"reasoning":0,"cache":{"read":0,"write":0}},"time":{"created":1791367200000,"completed":1791367201000}}'),
    ('msg_blog_2', 'ses_blog', 'compaction', 6, 1794823200000, 1794823200000,
     '{}');

INSERT INTO credential VALUES
    ('cred_deepseek', 'deepseek', 'Imported',
     '{"type":"key","key":"sk-made-up-deepseek-fixture-0000"}');
