CREATE TABLE users (
    id uuid PRIMARY KEY NOT NULL,
    first_name varchar NOT NULL,
    last_name varchar NOT NULL,
    birth_date date NOT NULL,
    gender varchar(7) NOT NULL,
    city varchar NOT NULL
);

CREATE TABLE interest (
    id uuid PRIMARY KEY NOT NULL,
    user_id uuid REFERENCES users(id) NOT NULL,
    name varchar NOT NULL,
    description varchar NOT NULL
);

CREATE TABLE auth (
    id uuid PRIMARY KEY,
    user_id uuid REFERENCES users(id) NOT NULL,
    login varchar NOT NULL UNIQUE,
    password varchar NOT NULL
);

CREATE TABLE sessions (
    session_id varchar PRIMARY KEY
);