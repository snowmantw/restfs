#![feature(proc_macro, specialization)]
#![feature(rustc_private)]
#![feature(libc)]
extern crate pyo3;
extern crate hyper;
extern crate fuse;
extern crate env_logger;
extern crate libc;
extern crate time;

use std::collections::HashMap;
use std::ffi::OsStr;
use libc::ENOENT;
use time::Timespec;
use fuse::{FileType, FileAttr, Filesystem, Request, ReplyData, ReplyEntry, ReplyAttr, ReplyDirectory};

use pyo3::prelude::*;
use pyo3::{pymodinit, pyclass, pymethods};

use hyper::Client;
use hyper::rt::{self, Future, Stream};

enum HTTPVerb {
    GET,
    HEAD,
    POST,
    PUT,
    DELETE,
    CONNECT,
    OPTIONS,
    TRACE,
    PATCH
}

#[pyclass(subclass)]
struct Adapter {
    #[prop(get, set)]
    debug: bool,
    token: PyToken
}

#[pymethods]
impl Adapter {
    #[new]
    fn __new__(obj: &PyRawObject, _debug: Option<bool>) -> PyResult<()> {
        let debug = match _debug {
            Some(x) => x,
            None => false,
        };
        obj.init(|token| {
            Adapter {
                debug,
                token
            }
        })
    }

    // TODO: decide to turn verb to Python Enum (complicated to introduced) or
    // keep as Rust enum (cannot export back to python for customed overriding method)
    //
    // NOTE: Input Arguments should be Py* types (with FromPyObject)
    // NOTE: Output PyResult should be in Rust (with IntoPyObject)
    // XXX: Therefore, this default method need to copy all from headers and convert it
    // to a new HashMap in Rust, as the result.
    //
    // NOTE: PyResult + HashMap has conversion issue should avoid:
    // https://www.reddit.com/r/Python/comments/8svfkz/writing_python_extensions_in_rust_using_pyo3/
    fn precommit(&self, verb: u8, headers: &PyDict, url: &str, body: &str) ->
        PyResult<(u8, HashMap<String, String>, String, String)>
    {
        let hheaders = from_py_dict(headers);
        // Overriding method for specific RESTful service should change the URL and body
        // if it is necessary.
        Ok((verb, hheaders, String::from(url), String::from(body)))
    }

    // After receiving the response from server: if it is 200 then create/overwrite a file
    // according to the [1] String of PyResult here returned. Log-only for other status code
    //
    // Overriding this method to provide customed content to write to the file.
    // Customed logic can know the type of receiving for Accept header get set
    // and catched in precommit.
    fn postcommit(&self, statuscode: u8, response: &str) ->
        PyResult<(u8, String)>
    {
        Ok((statuscode, String::from(response)))
    }

    // NOTE: since it is failed to acquire GIL from Rust side not connected with PyO3 (like defined
    // as RestFS::commit), we have no choice but define commit function here.
    //
    // The GIL failed because of pythread init assertion failure. Ref:
    // https://github.com/PyO3/pyo3/blob/master/src/pythonrun.rs#L42
    //
    // TODO: turn the file operation (fop) to enum.
    fn commit(&self, fop: u8, path: &str) ->
        PyResult<()>
    {
        Ok(())
    }
}

fn from_py_dict(pd: &PyDict) -> HashMap<String, String> {
    let mut hmap = HashMap::new();
    let hitems = pd.copy().unwrap().into_iter();
    for h in hitems {
        let (pyhk, pyhv) = h;
        let hk = pyhk.extract::<&str>().unwrap();
        let hv = pyhv.extract::<&str>().unwrap();
        hmap.insert(String::from(hk), String::from(hv));
    }
    return hmap;
}

#[pymodinit]
fn restfslib(_py: Python, m: &PyModule) -> PyResult<()> {
    #[pyfn(m, "mount")]
    // ``#[pyfn()]` converts the arguments from Python objects to Rust values
    // and the Rust return value back into a Python object.
    fn mount_py(_py: Python, madapter: PyObject, mpath: String) -> PyResult<()> {
        // TODO
        // need to handle passing GIL/py here to the RestFS structure (for calling its methods)
        Ok(mount(madapter.extract(_py).unwrap(), &mpath))
    }

    // NOTE: need this to add the class to the module.
    // https://docs.rs/pyo3/0.2.5/pyo3/struct.PyModule.html#method.add_class
    m.add_class::<Adapter>()?;

    Ok(())
}

// NOTE: Need pass the Adapter since we need to call methods after mouting for operations.
fn mount(madapter: &Adapter, mpath: &str) -> () {
    env_logger::init();
    let mountpoint = mpath; 
    let options = ["-o", "ro", "-o", "fsname=hello"]
        .iter()
        .map(|o| o.as_ref())
        .collect::<Vec<&OsStr>>();

    // This will hold the main process until it get `umount`:
    // better to call umount if Python get KeyInterrupt from Python side.
    fuse::mount(RestFS { adapter: madapter, table: HashMap::new() }, &mountpoint, &options).unwrap();
}
const TTL: Timespec = Timespec { sec: 1, nsec: 0 };                     // 1 second

const CREATE_TIME: Timespec = Timespec { sec: 1381237736, nsec: 0 };    // 2013-10-08 08:56

const HELLO_DIR_ATTR: FileAttr = FileAttr {
    ino: 1,
    size: 0,
    blocks: 0,
    atime: CREATE_TIME,
    mtime: CREATE_TIME,
    ctime: CREATE_TIME,
    crtime: CREATE_TIME,
    kind: FileType::Directory,
    perm: 0o755,
    nlink: 2,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
};

const HELLO_TXT_CONTENT: &'static str = "Hello World!\n";

const HELLO_TXT_ATTR: FileAttr = FileAttr {
    ino: 2,
    size: 13,
    blocks: 1,
    atime: CREATE_TIME,
    mtime: CREATE_TIME,
    ctime: CREATE_TIME,
    crtime: CREATE_TIME,
    kind: FileType::RegularFile,
    perm: 0o644,
    nlink: 1,
    uid: 501,
    gid: 20,
    rdev: 0,
    flags: 0,
};

enum FileContent {
    Directory(Vec<u64>),
    File(String)
}

struct FileNode {
    name: String,
    content: FileContent
}

struct RestFS<'a> {
    adapter: &'a Adapter,
    table: HashMap<u64, FileNode>
}

/**
 * For the inode:
 *
 * 1 means `.` in the fs
 * 2 means `hello.txt`
 *
 * We need a tracking list for each file/directory created because of restful operations.
 *
 * For a practical fs design about filenames:
 * https://unix.stackexchange.com/questions/117325/where-are-filenames-stored-on-a-filesystem
 *
 * But using an universal table is enough for this case. This is a disk in memory, and we use a
 * table to store all the things including content. There is two keys: inode and name, while the
 * content is either a String or an inode (if it is directory).
 *
 */

impl<'a> Filesystem for RestFS<'a> {

    fn lookup(&mut self, _req: &Request, parent: u64, name: &OsStr, reply: ReplyEntry) {

        self.adapter.commit(0, name.to_str().unwrap());
        if parent == 1 && name.to_str() == Some("hello.txt") {
            reply.entry(&TTL, &HELLO_TXT_ATTR, 0);
        } else {
            reply.error(ENOENT);
        }
    }

    fn getattr(&mut self, _req: &Request, ino: u64, reply: ReplyAttr) {
        match ino {
            1 => reply.attr(&TTL, &HELLO_DIR_ATTR),
            2 => reply.attr(&TTL, &HELLO_TXT_ATTR),
            _ => reply.error(ENOENT),
        }
    }

    fn read(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, _size: u32, reply: ReplyData) {
        if ino == 2 {
            reply.data(&HELLO_TXT_CONTENT.as_bytes()[offset as usize..]);
        } else {
            reply.error(ENOENT);
        }
    }

    fn readdir(&mut self, _req: &Request, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        if ino == 1 {
            if offset == 0 {
                reply.add(1, 0, FileType::Directory, ".");
                reply.add(1, 1, FileType::Directory, "..");
                reply.add(2, 2, FileType::RegularFile, "hello.txt");
            }
            reply.ok();
        } else {
            reply.error(ENOENT);
        }
    }
}

