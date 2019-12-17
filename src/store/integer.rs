// Copyright 2018-2019 Mozilla
//
// Licensed under the Apache License, Version 2.0 (the "License"); you may not use
// this file except in compliance with the License. You may obtain a copy of the
// License at http://www.apache.org/licenses/LICENSE-2.0
// Unless required by applicable law or agreed to in writing, software distributed
// under the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR
// CONDITIONS OF ANY KIND, either express or implied. See the License for the
// specific language governing permissions and limitations under the License.

use std::marker::PhantomData;

use crate::backend::{
    BackendDatabase,
    BackendRwTransaction,
};
use crate::error::StoreError;
use crate::readwrite::{
    Readable,
    Writer,
};
use crate::store::keys::{
    Key,
    PrimitiveInt,
};
use crate::store::single::SingleStore;
use crate::value::Value;

type EmptyResult = Result<(), StoreError>;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct IntegerStore<D, K> {
    inner: SingleStore<D>,
    phantom: PhantomData<K>,
}

impl<D, K> IntegerStore<D, K>
where
    D: BackendDatabase,
    K: PrimitiveInt,
{
    pub(crate) fn new(db: D) -> IntegerStore<D, K> {
        IntegerStore {
            inner: SingleStore::new(db),
            phantom: PhantomData,
        }
    }

    pub fn get<'r, R>(&self, reader: &'r R, k: K) -> Result<Option<Value<'r>>, StoreError>
    where
        R: Readable<'r, Database = D>,
    {
        self.inner.get(reader, Key::new(&k)?)
    }

    pub fn put<T>(&self, writer: &mut Writer<T>, k: K, v: &Value) -> EmptyResult
    where
        T: BackendRwTransaction<Database = D>,
    {
        self.inner.put(writer, Key::new(&k)?, v)
    }

    pub fn delete<T>(&self, writer: &mut Writer<T>, k: K) -> EmptyResult
    where
        T: BackendRwTransaction<Database = D>,
    {
        self.inner.delete(writer, Key::new(&k)?)
    }

    pub fn clear<T>(&self, writer: &mut Writer<T>) -> EmptyResult
    where
        T: BackendRwTransaction<Database = D>,
    {
        self.inner.clear(writer)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempfile::Builder;

    use super::*;
    use crate::*;

    #[test]
    fn test_integer_keys() {
        let root = Builder::new().prefix("test_integer_keys").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        let k = Rkv::new::<backend::Lmdb>(root.path()).expect("new succeeded");
        let s = k.open_integer("s", StoreOptions::create()).expect("open");

        macro_rules! test_integer_keys {
            ($type:ty, $key:expr) => {{
                let mut writer = k.write().expect("writer");

                s.put(&mut writer, $key, &Value::Str("hello!")).expect("write");
                assert_eq!(s.get(&writer, $key).expect("read"), Some(Value::Str("hello!")));
                writer.commit().expect("committed");

                let reader = k.read().expect("reader");
                assert_eq!(s.get(&reader, $key).expect("read"), Some(Value::Str("hello!")));
            }};
        }

        test_integer_keys!(u32, std::u32::MIN);
        test_integer_keys!(u32, std::u32::MAX);
    }

    #[test]
    fn test_clear() {
        let root = Builder::new().prefix("test_integer_clear").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        let k = Rkv::new::<backend::Lmdb>(root.path()).expect("new succeeded");
        let s = k.open_integer("s", StoreOptions::create()).expect("open");

        {
            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 2, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 3, &Value::Str("hello!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 2).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 3).expect("read"), Some(Value::Str("hello!")));
            writer.commit().expect("committed");
        }

        {
            let mut writer = k.write().expect("writer");
            s.clear(&mut writer).expect("cleared");
            writer.commit().expect("committed");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 1).expect("read"), None);
            assert_eq!(s.get(&reader, 2).expect("read"), None);
            assert_eq!(s.get(&reader, 3).expect("read"), None);
        }
    }

    #[test]
    fn test_dup() {
        let root = Builder::new().prefix("test_integer_dup").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        let k = Rkv::new::<backend::Lmdb>(root.path()).expect("new succeeded");
        let s = k.open_integer("s", StoreOptions::create()).expect("open");

        {
            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 1, &Value::Str("foo!")).expect("write");
            s.put(&mut writer, 1, &Value::Str("bar!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("bar!")));
            assert_eq!(s.get(&writer, 2).expect("read"), None);
            assert_eq!(s.get(&writer, 3).expect("read"), None);
            writer.commit().expect("committed");
        }

        {
            let mut writer = k.write().expect("writer");
            s.clear(&mut writer).expect("cleared");
            writer.commit().expect("committed");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 1).expect("read"), None);
            assert_eq!(s.get(&reader, 2).expect("read"), None);
            assert_eq!(s.get(&reader, 3).expect("read"), None);
        }
    }

    #[test]
    fn test_del() {
        let root = Builder::new().prefix("test_integer_del").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        let k = Rkv::new::<backend::Lmdb>(root.path()).expect("new succeeded");
        let s = k.open_integer("s", StoreOptions::create()).expect("open");

        {
            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 1, &Value::Str("foo!")).expect("write");
            s.put(&mut writer, 1, &Value::Str("bar!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("bar!")));
            assert_eq!(s.get(&writer, 2).expect("read"), None);
            assert_eq!(s.get(&writer, 3).expect("read"), None);
            writer.commit().expect("committed");
        }

        {
            let mut writer = k.write().expect("writer");
            s.delete(&mut writer, 1).expect("deleted");
            writer.commit().expect("committed");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 1).expect("read"), None);
        }

        {
            let mut writer = k.write().expect("writer");
            s.delete(&mut writer, 2).expect_err("not deleted");
            writer.commit().expect("committed");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 2).expect("read"), None);
        }

        {
            let mut writer = k.write().expect("writer");
            s.delete(&mut writer, 3).expect_err("not deleted");
            writer.commit().expect("committed");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 3).expect("read"), None);
        }
    }

    #[test]
    fn test_persist() {
        let root = Builder::new().prefix("test_integer_persist").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        {
            let k = Rkv::new::<backend::Lmdb>(root.path()).expect("new succeeded");
            let s = k.open_integer("s", StoreOptions::create()).expect("open");

            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 2, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 3, &Value::Str("hello!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 2).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 3).expect("read"), Some(Value::Str("hello!")));
            writer.commit().expect("committed");
        }

        {
            let k = Rkv::new::<backend::Lmdb>(root.path()).expect("new succeeded");
            let s = k.open_integer("s", StoreOptions::create()).expect("open");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&reader, 2).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&reader, 3).expect("read"), Some(Value::Str("hello!")));
        }
    }

    #[test]
    fn test_intertwine_read_write() {
        let root = Builder::new().prefix("test_integer_intertwine_read_write").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        let k = Rkv::new::<backend::Lmdb>(root.path()).expect("new succeeded");
        let s = k.open_integer("s", StoreOptions::create()).expect("open");

        {
            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 2).expect("read"), None);
            assert_eq!(s.get(&writer, 3).expect("read"), None);
            writer.commit().expect("committed");
        }

        let reader = k.read().expect("reader");
        let mut writer = k.write().expect("writer");

        {
            assert_eq!(s.get(&reader, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&reader, 2).expect("read"), None);
            assert_eq!(s.get(&reader, 3).expect("read"), None);
        }

        {
            s.put(&mut writer, 1, &Value::Str("goodbye!")).expect("write");
            s.put(&mut writer, 2, &Value::Str("goodbye!")).expect("write");
            s.put(&mut writer, 3, &Value::Str("goodbye!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("goodbye!")));
            assert_eq!(s.get(&writer, 2).expect("read"), Some(Value::Str("goodbye!")));
            assert_eq!(s.get(&writer, 3).expect("read"), Some(Value::Str("goodbye!")));
            writer.commit().expect("committed");
        }

        {
            assert_eq!(s.get(&reader, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&reader, 2).expect("read"), None);
            assert_eq!(s.get(&reader, 3).expect("read"), None);
        }

        {
            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 2).expect("read"), Some(Value::Str("goodbye!")));
            assert_eq!(s.get(&writer, 3).expect("read"), Some(Value::Str("goodbye!")));
            writer.commit().expect("committed");
        }

        {
            let reader = k.write().expect("reader");
            assert_eq!(s.get(&reader, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&reader, 2).expect("read"), Some(Value::Str("goodbye!")));
            assert_eq!(s.get(&reader, 3).expect("read"), Some(Value::Str("goodbye!")));
            reader.commit().expect("committed");
        }
    }
}

#[cfg(test)]
mod tests_safe {
    use std::fs;
    use tempfile::Builder;

    use super::*;
    use crate::*;

    #[test]
    fn test_integer_keys() {
        let root = Builder::new().prefix("test_integer_keys").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        let k = Rkv::new::<backend::SafeMode>(root.path()).expect("new succeeded");
        let s = k.open_integer("s", StoreOptions::create()).expect("open");

        macro_rules! test_integer_keys {
            ($type:ty, $key:expr) => {{
                let mut writer = k.write().expect("writer");

                s.put(&mut writer, $key, &Value::Str("hello!")).expect("write");
                assert_eq!(s.get(&writer, $key).expect("read"), Some(Value::Str("hello!")));
                writer.commit().expect("committed");

                let reader = k.read().expect("reader");
                assert_eq!(s.get(&reader, $key).expect("read"), Some(Value::Str("hello!")));
            }};
        }

        test_integer_keys!(u32, std::u32::MIN);
        test_integer_keys!(u32, std::u32::MAX);
    }

    #[test]
    fn test_clear() {
        let root = Builder::new().prefix("test_integer_clear").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        let k = Rkv::new::<backend::SafeMode>(root.path()).expect("new succeeded");
        let s = k.open_integer("s", StoreOptions::create()).expect("open");

        {
            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 2, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 3, &Value::Str("hello!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 2).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 3).expect("read"), Some(Value::Str("hello!")));
            writer.commit().expect("committed");
        }

        {
            let mut writer = k.write().expect("writer");
            s.clear(&mut writer).expect("cleared");
            writer.commit().expect("committed");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 1).expect("read"), None);
            assert_eq!(s.get(&reader, 2).expect("read"), None);
            assert_eq!(s.get(&reader, 3).expect("read"), None);
        }
    }

    #[test]
    fn test_dup() {
        let root = Builder::new().prefix("test_integer_dup").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        let k = Rkv::new::<backend::SafeMode>(root.path()).expect("new succeeded");
        let s = k.open_integer("s", StoreOptions::create()).expect("open");

        {
            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 1, &Value::Str("foo!")).expect("write");
            s.put(&mut writer, 1, &Value::Str("bar!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("bar!")));
            assert_eq!(s.get(&writer, 2).expect("read"), None);
            assert_eq!(s.get(&writer, 3).expect("read"), None);
            writer.commit().expect("committed");
        }

        {
            let mut writer = k.write().expect("writer");
            s.clear(&mut writer).expect("cleared");
            writer.commit().expect("committed");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 1).expect("read"), None);
            assert_eq!(s.get(&reader, 2).expect("read"), None);
            assert_eq!(s.get(&reader, 3).expect("read"), None);
        }
    }

    #[test]
    fn test_del() {
        let root = Builder::new().prefix("test_integer_del").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        let k = Rkv::new::<backend::SafeMode>(root.path()).expect("new succeeded");
        let s = k.open_integer("s", StoreOptions::create()).expect("open");

        {
            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 1, &Value::Str("foo!")).expect("write");
            s.put(&mut writer, 1, &Value::Str("bar!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("bar!")));
            assert_eq!(s.get(&writer, 2).expect("read"), None);
            assert_eq!(s.get(&writer, 3).expect("read"), None);
            writer.commit().expect("committed");
        }

        {
            let mut writer = k.write().expect("writer");
            s.delete(&mut writer, 1).expect("deleted");
            writer.commit().expect("committed");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 1).expect("read"), None);
        }

        {
            let mut writer = k.write().expect("writer");
            s.delete(&mut writer, 2).expect_err("not deleted");
            writer.commit().expect("committed");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 2).expect("read"), None);
        }

        {
            let mut writer = k.write().expect("writer");
            s.delete(&mut writer, 3).expect_err("not deleted");
            writer.commit().expect("committed");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 3).expect("read"), None);
        }
    }

    #[test]
    fn test_persist() {
        let root = Builder::new().prefix("test_integer_persist").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        {
            let k = Rkv::new::<backend::SafeMode>(root.path()).expect("new succeeded");
            let s = k.open_integer("s", StoreOptions::create()).expect("open");

            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 2, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 3, &Value::Str("hello!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 2).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 3).expect("read"), Some(Value::Str("hello!")));
            writer.commit().expect("committed");
        }

        {
            let k = Rkv::new::<backend::SafeMode>(root.path()).expect("new succeeded");
            let s = k.open_integer("s", StoreOptions::create()).expect("open");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&reader, 2).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&reader, 3).expect("read"), Some(Value::Str("hello!")));
        }
    }

    #[test]
    fn test_intertwine_read_write() {
        let root = Builder::new().prefix("test_integer_intertwine_read_write").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        let k = Rkv::new::<backend::SafeMode>(root.path()).expect("new succeeded");
        let s = k.open_integer("s", StoreOptions::create()).expect("open");

        {
            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 2).expect("read"), None);
            assert_eq!(s.get(&writer, 3).expect("read"), None);
            writer.commit().expect("committed");
        }

        let reader = k.read().expect("reader");
        let mut writer = k.write().expect("writer");

        {
            assert_eq!(s.get(&reader, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&reader, 2).expect("read"), None);
            assert_eq!(s.get(&reader, 3).expect("read"), None);
        }

        {
            s.put(&mut writer, 1, &Value::Str("goodbye!")).expect("write");
            s.put(&mut writer, 2, &Value::Str("goodbye!")).expect("write");
            s.put(&mut writer, 3, &Value::Str("goodbye!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("goodbye!")));
            assert_eq!(s.get(&writer, 2).expect("read"), Some(Value::Str("goodbye!")));
            assert_eq!(s.get(&writer, 3).expect("read"), Some(Value::Str("goodbye!")));
            writer.commit().expect("committed");
        }

        {
            assert_eq!(s.get(&reader, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&reader, 2).expect("read"), None);
            assert_eq!(s.get(&reader, 3).expect("read"), None);
        }

        {
            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 2).expect("read"), Some(Value::Str("goodbye!")));
            assert_eq!(s.get(&writer, 3).expect("read"), Some(Value::Str("goodbye!")));
            writer.commit().expect("committed");
        }

        {
            let reader = k.write().expect("reader");
            assert_eq!(s.get(&reader, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&reader, 2).expect("read"), Some(Value::Str("goodbye!")));
            assert_eq!(s.get(&reader, 3).expect("read"), Some(Value::Str("goodbye!")));
            reader.commit().expect("committed");
        }
    }
}

#[cfg(test)]
mod tests_rusqlite {
    use std::fs;
    use tempfile::Builder;

    use super::*;
    use crate::*;

    use rusqlite::{ Connection, NO_PARAMS, Result };

    #[test]
    fn test_rusqlite_sample_usage() {
        match print_dir_contents() {
            Ok(_) => assert!(true),
            Err(_) => assert!(false),
        }
    }

    fn print_dir_contents() -> Result<()> {

        // Temporary database file for the test:
        Builder::new().prefix("test_integer_keys").tempdir().expect("tempdir");
        let file = Builder::new().prefix("test_sqlite").tempfile().expect("tempfile");
        let conn = Connection::open(file.path()).unwrap();

        // Or, put a db in working dir to inspect later:
        // let conn = Connection::open("myfile.db").unwrap();

        conn.execute("CREATE TABLE IF NOT EXISTS wordcount(
            word TEXT PRIMARY KEY,
            cnt INTEGER
            ) WITHOUT ROWID
        ", NO_PARAMS).unwrap();

        conn.execute("CREATE TABLE IF NOT EXISTS person (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT NOT NULL
          )", NO_PARAMS).unwrap();

        let first_name: String = "Example".to_string();
        let first_email: String = "example@example.org".to_string();
        conn.execute("INSERT INTO person (name, email) VALUES (?1, ?2)",
            &[&first_name, &first_email]).unwrap();

        let second_name: String = "Foo".to_string();
        let second_email: String = "foo@example.org".to_string();
        conn.execute("INSERT INTO person (name, email) VALUES (?1, ?2)",
            &[&second_name, &second_email]).unwrap();

        #[derive(Debug)]
        struct Person {
            id: i32,
            name: String,
            email: String,
        }

        let mut stmt = conn.prepare("SELECT id, name, email FROM person")?;
        let mut person_iter = stmt.query_map(NO_PARAMS, |row| {
            Ok(Person {
                id: row.get(0)?,
                name: row.get(1)?,
                email: row.get(2)?,
            })
        })?;

        let first_person = person_iter.next().unwrap().unwrap();
        assert_eq!(first_person.name, first_name);
        assert_eq!(first_person.email, first_email);
        assert_eq!(first_person.id, 1);

        let second_person = person_iter.next().unwrap().unwrap();
        assert_eq!(second_person.name, second_name);
        assert_eq!(second_person.email, second_email);
        assert_eq!(second_person.id, 2);
        // for person in person_iter {
        //     println!("Found person {:?}", person.unwrap());
        // }

        Ok(())
    }
}

// XXX: DRY these tests by having one `mod` and looping through three backends?
#[cfg(test)]
mod tests_sqlite {
    use std::fs;
    use tempfile::Builder;

    use super::*;
    use crate::*;

    #[test]
    fn test_integer_keys() {
        let root = Builder::new().prefix("test_integer_keys").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        let k = Rkv::new::<backend::Sqlite>(root.path()).expect("new succeeded");
        let s = k.open_integer("s", StoreOptions::create()).expect("open");

        macro_rules! test_integer_keys {
            ($type:ty, $key:expr) => {{
                let mut writer = k.write().expect("writer");

                s.put(&mut writer, $key, &Value::Str("hello!")).expect("write");
                assert_eq!(s.get(&writer, $key).expect("read"), Some(Value::Str("hello!")));
                writer.commit().expect("committed");

                let reader = k.read().expect("reader");
                assert_eq!(s.get(&reader, $key).expect("read"), Some(Value::Str("hello!")));
            }};
        }

        test_integer_keys!(u32, std::u32::MIN);
        test_integer_keys!(u32, std::u32::MAX);
    }

    #[test]
    fn test_clear() {
        let root = Builder::new().prefix("test_integer_clear").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        let k = Rkv::new::<backend::Sqlite>(root.path()).expect("new succeeded");
        let s = k.open_integer("s", StoreOptions::create()).expect("open");

        {
            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 2, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 3, &Value::Str("hello!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 2).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 3).expect("read"), Some(Value::Str("hello!")));
            writer.commit().expect("committed");
        }

        {
            let mut writer = k.write().expect("writer");
            s.clear(&mut writer).expect("cleared");
            writer.commit().expect("committed");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 1).expect("read"), None);
            assert_eq!(s.get(&reader, 2).expect("read"), None);
            assert_eq!(s.get(&reader, 3).expect("read"), None);
        }
    }

    #[test]
    fn test_dup() {
        let root = Builder::new().prefix("test_integer_dup").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        let k = Rkv::new::<backend::Sqlite>(root.path()).expect("new succeeded");
        let s = k.open_integer("s", StoreOptions::create()).expect("open");

        {
            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 1, &Value::Str("foo!")).expect("write");
            s.put(&mut writer, 1, &Value::Str("bar!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("bar!")));
            assert_eq!(s.get(&writer, 2).expect("read"), None);
            assert_eq!(s.get(&writer, 3).expect("read"), None);
            writer.commit().expect("committed");
        }

        {
            let mut writer = k.write().expect("writer");
            s.clear(&mut writer).expect("cleared");
            writer.commit().expect("committed");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 1).expect("read"), None);
            assert_eq!(s.get(&reader, 2).expect("read"), None);
            assert_eq!(s.get(&reader, 3).expect("read"), None);
        }
    }

    #[test]
    fn test_del() {
        let root = Builder::new().prefix("test_integer_del").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        let k = Rkv::new::<backend::Sqlite>(root.path()).expect("new succeeded");
        let s = k.open_integer("s", StoreOptions::create()).expect("open");

        {
            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 1, &Value::Str("foo!")).expect("write");
            s.put(&mut writer, 1, &Value::Str("bar!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("bar!")));
            assert_eq!(s.get(&writer, 2).expect("read"), None);
            assert_eq!(s.get(&writer, 3).expect("read"), None);
            writer.commit().expect("committed");
        }

        {
            let mut writer = k.write().expect("writer");
            s.delete(&mut writer, 1).expect("deleted");
            writer.commit().expect("committed");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 1).expect("read"), None);
        }

        {
            let mut writer = k.write().expect("writer");
            s.delete(&mut writer, 2).expect_err("not deleted");
            writer.commit().expect("committed");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 2).expect("read"), None);
        }

        {
            let mut writer = k.write().expect("writer");
            s.delete(&mut writer, 3).expect_err("not deleted");
            writer.commit().expect("committed");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 3).expect("read"), None);
        }
    }

    #[test]
    fn test_persist() {
        let root = Builder::new().prefix("test_integer_persist").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        {
            let k = Rkv::new::<backend::Sqlite>(root.path()).expect("new succeeded");
            let s = k.open_integer("s", StoreOptions::create()).expect("open");

            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 2, &Value::Str("hello!")).expect("write");
            s.put(&mut writer, 3, &Value::Str("hello!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 2).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 3).expect("read"), Some(Value::Str("hello!")));
            writer.commit().expect("committed");
        }

        {
            let k = Rkv::new::<backend::Sqlite>(root.path()).expect("new succeeded");
            let s = k.open_integer("s", StoreOptions::create()).expect("open");

            let reader = k.read().expect("reader");
            assert_eq!(s.get(&reader, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&reader, 2).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&reader, 3).expect("read"), Some(Value::Str("hello!")));
        }
    }

    #[test]
    fn test_intertwine_read_write() {
        let root = Builder::new().prefix("test_integer_intertwine_read_write").tempdir().expect("tempdir");
        fs::create_dir_all(root.path()).expect("dir created");

        let k = Rkv::new::<backend::Sqlite>(root.path()).expect("new succeeded");
        let s = k.open_integer("s", StoreOptions::create()).expect("open");

        {
            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 2).expect("read"), None);
            assert_eq!(s.get(&writer, 3).expect("read"), None);
            writer.commit().expect("committed");
        }

        let reader = k.read().expect("reader");
        let mut writer = k.write().expect("writer");

        {
            assert_eq!(s.get(&reader, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&reader, 2).expect("read"), None);
            assert_eq!(s.get(&reader, 3).expect("read"), None);
        }

        {
            s.put(&mut writer, 1, &Value::Str("goodbye!")).expect("write");
            s.put(&mut writer, 2, &Value::Str("goodbye!")).expect("write");
            s.put(&mut writer, 3, &Value::Str("goodbye!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("goodbye!")));
            assert_eq!(s.get(&writer, 2).expect("read"), Some(Value::Str("goodbye!")));
            assert_eq!(s.get(&writer, 3).expect("read"), Some(Value::Str("goodbye!")));
            writer.commit().expect("committed");
        }

        {
            assert_eq!(s.get(&reader, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&reader, 2).expect("read"), None);
            assert_eq!(s.get(&reader, 3).expect("read"), None);
        }

        {
            let mut writer = k.write().expect("writer");
            s.put(&mut writer, 1, &Value::Str("hello!")).expect("write");
            assert_eq!(s.get(&writer, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&writer, 2).expect("read"), Some(Value::Str("goodbye!")));
            assert_eq!(s.get(&writer, 3).expect("read"), Some(Value::Str("goodbye!")));
            writer.commit().expect("committed");
        }

        {
            let reader = k.write().expect("reader");
            assert_eq!(s.get(&reader, 1).expect("read"), Some(Value::Str("hello!")));
            assert_eq!(s.get(&reader, 2).expect("read"), Some(Value::Str("goodbye!")));
            assert_eq!(s.get(&reader, 3).expect("read"), Some(Value::Str("goodbye!")));
            reader.commit().expect("committed");
        }
    }
}
