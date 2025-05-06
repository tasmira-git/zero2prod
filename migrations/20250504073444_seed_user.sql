-- Add migration script here
INSERT INTO users (user_id, username, password_hash)
VALUES (
    'a6de010c-e9c2-457d-bbac-1a0eddc74ddd',
    'admin',
    '$argon2id$v=19$m=19456,t=2,p=1$lN6i+abo5BFuNLw5oBGoog$FdrpeOMW4CthDlx5pNRD0YJV1YIUQBtrSJs/1Y2bv4s' 
);
