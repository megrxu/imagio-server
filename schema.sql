CREATE TABLE IF NOT EXISTS images (
  id integer PRIMARY KEY AUTOINCREMENT,
  mime text NOT NULL,
  category text NOT NULL,
  uuid text NOT NULL,
  create_time datetime NOT NULL
);