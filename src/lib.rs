//! See https://www.kernel.org/pub/linux/utils/util-linux/v2.21/libblkid-docs/index.html
//! for the reference manual to the FFI bindings
extern crate blkid_sys;
extern crate errno;
extern crate libc;

use std::error::Error as err;
use std::ffi::{CStr, CString, IntoStringError, NulError};
use std::fmt;
use std::io::Error;
use std::path::Path;
use std::ptr;
use std::string::FromUtf8Error;

use blkid_sys::*;
use errno::errno;

/// Custom error handling for the library
#[derive(Debug)]
pub enum BlkidError {
    FromUtf8Error(FromUtf8Error),
    NulError(NulError),
    Error(String),
    IoError(Error),
    IntoStringError(IntoStringError),
}

impl fmt::Display for BlkidError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.description())
    }
}

impl err for BlkidError {
    fn description(&self) -> &str {
        match *self {
            BlkidError::FromUtf8Error(ref e) => e.description(),
            BlkidError::NulError(ref e) => e.description(),
            BlkidError::Error(ref e) => &e,
            BlkidError::IoError(ref e) => e.description(),
            BlkidError::IntoStringError(ref e) => e.description(),
        }
    }
    fn cause(&self) -> Option<&err> {
        match *self {
            BlkidError::FromUtf8Error(ref e) => e.cause(),
            BlkidError::NulError(ref e) => e.cause(),
            BlkidError::Error(_) => None,
            BlkidError::IoError(ref e) => e.cause(),
            BlkidError::IntoStringError(ref e) => e.cause(),
        }
    }
}
impl BlkidError {
    /// Create a new BlkidError with a String message
    fn new(err: String) -> BlkidError {
        BlkidError::Error(err)
    }

    /// Convert a BlkidError into a String representation.
    pub fn to_string(&self) -> String {
        match *self {
            BlkidError::FromUtf8Error(ref err) => err.utf8_error().to_string(),
            BlkidError::NulError(ref err) => err.description().to_string(),
            BlkidError::Error(ref err) => err.to_string(),
            BlkidError::IoError(ref err) => err.description().to_string(),
            BlkidError::IntoStringError(ref err) => err.description().to_string(),
        }
    }
}

impl From<NulError> for BlkidError {
    fn from(err: NulError) -> BlkidError {
        BlkidError::NulError(err)
    }
}

impl From<FromUtf8Error> for BlkidError {
    fn from(err: FromUtf8Error) -> BlkidError {
        BlkidError::FromUtf8Error(err)
    }
}
impl From<IntoStringError> for BlkidError {
    fn from(err: IntoStringError) -> BlkidError {
        BlkidError::IntoStringError(err)
    }
}
impl From<Error> for BlkidError {
    fn from(err: Error) -> BlkidError {
        BlkidError::IoError(err)
    }
}

fn get_error() -> String {
    let error = errno();
    format!("{}", error)
}

pub struct BlkId {
    probe: blkid_probe,
}

impl BlkId {
    pub fn new(file: &Path) -> Result<BlkId, BlkidError> {
        let path = try!(CString::new(file.as_os_str().to_string_lossy().as_ref()));
        unsafe {
            // pub fn blkid_do_probe(pr: blkid_probe) -> ::std::os::raw::c_int;
            // pub fn blkid_do_safeprobe(pr: blkid_probe) -> ::std::os::raw::c_int;
            // pub fn blkid_do_fullprobe(pr: blkid_probe) -> ::std::os::raw::c_int;
            let probe = blkid_new_probe_from_filename(path.as_ptr());
            Ok(BlkId { probe: probe })
        }
    }
    /// Calls probing functions in all enabled chains. The superblocks chain is enabled by
    /// default. The blkid_do_probe() stores result from only one probing function.
    /// It's necessary to call this routine in a loop to get results from all probing functions
    /// in all chains. The probing is reset by blkid_reset_probe() or by filter functions.
    /// This is string-based NAME=value interface only.
    pub fn do_probe(&self) -> Result<(), BlkidError> {
        unsafe {
            let ret_code = blkid_do_probe(self.probe);
            if ret_code < 0 {
                return Err(BlkidError::new(get_error()));
            }
        }
        Ok(())
    }
    pub fn lookup_value(&self, name: &str) -> Result<String, BlkidError> {
        let name = try!(CString::new(name));
        let mut data_ptr: *const ::libc::c_char = ptr::null();
        let mut len = 0;
        unsafe {
            let ret_code =
                blkid_probe_lookup_value(self.probe, name.as_ptr(), &mut data_ptr, &mut len);
            if ret_code < 0 {
                return Err(BlkidError::new(get_error()));
            }
            let data_value = CStr::from_ptr(data_ptr as *const ::libc::c_char);
            Ok(data_value.to_string_lossy().into_owned())
        }
    }
    pub fn has_value(&self, name: &str) -> Result<bool, BlkidError> {
        let name = try!(CString::new(name));

        unsafe {
            let ret_code = blkid_probe_has_value(self.probe, name.as_ptr());
            if ret_code < 0 {
                return Err(BlkidError::new(get_error()));
            }
            match ret_code {
                0 => Ok(false),
                1 => Ok(true),
                _ => {
                    Err(BlkidError::new(format!("Unknown return code from \
                                                 blkid_probe_has_value: {}",
                                                ret_code)))
                }
            }
        }
    }
    /// number of values in probing result
    pub fn numof_values(&self) -> Result<i32, BlkidError> {
        unsafe {
            let ret_code = blkid_probe_numof_values(self.probe);
            if ret_code < 0 {
                return Err(BlkidError::new(get_error()));
            }
            Ok(ret_code)
        }
    }
    pub fn get_devno(&self) -> u64 {
        unsafe { blkid_probe_get_devno(self.probe) }
    }
    pub fn get_wholedisk_devno(&self) -> u64 {
        unsafe { blkid_probe_get_wholedisk_devno(self.probe) }
    }
    pub fn is_wholedisk(&self) -> Result<bool, BlkidError> {
        unsafe {
            let ret_code = blkid_probe_is_wholedisk(self.probe);
            match ret_code {
                0 => Ok(false),
                1 => Ok(true),
                _ => {
                    Err(BlkidError::new(format!("Unknown return code from \
                                                 blkid_probe_has_value: {}",
                                                ret_code)))
                }
            }
        }
    }
    pub fn get_size(&self) -> Result<i64, BlkidError> {
        unsafe {
            let ret_code = blkid_probe_get_size(self.probe);
            if ret_code < 0 {
                return Err(BlkidError::new(get_error()));
            }
            Ok(ret_code)
        }
    }
    pub fn get_offset(&self) -> Result<i64, BlkidError> {
        unsafe {
            let ret_code = blkid_probe_get_offset(self.probe);
            if ret_code < 0 {
                return Err(BlkidError::new(get_error()));
            }
            Ok(ret_code)
        }
    }
    pub fn get_sectorsize(&self) -> u32 {
        unsafe { blkid_probe_get_sectorsize(self.probe) }
    }
    pub fn get_sectors(&self) -> Result<i64, BlkidError> {
        unsafe {
            let ret_code = blkid_probe_get_sectors(self.probe);
            if ret_code < 0 {
                return Err(BlkidError::new(get_error()));
            }
            Ok(ret_code)
        }
    }
    pub fn get_fd(&self) -> Result<i32, BlkidError> {
        unsafe { Ok(blkid_probe_get_fd(self.probe)) }
    }
    pub fn known_fstype(&self, fstype: &str) -> Result<bool, BlkidError> {
        let fstype = try!(CString::new(fstype));
        unsafe {
            let ret_code = blkid_known_fstype(fstype.as_ptr());
            match ret_code {
                0 => Ok(false),
                1 => Ok(true),
                _ => {
                    Err(BlkidError::new(format!("Unknown return code from blkid_known_fstype: {}",
                                                ret_code)))
                }
            }
        }
    }
    /// Enables/disables the topology probing for non-binary interface.
    pub fn enable_topology(&self) -> Result<(), BlkidError> {
        unsafe {
            let ret_code = blkid_probe_enable_topology(self.probe, 1);
            if ret_code < 0 {
                return Err(BlkidError::new(get_error()));
            }
        }
        Ok(())
    }
    /// This is a binary interface for topology values. See also blkid_topology_* functions.
    /// This function is independent on blkid_do_[safe,full]probe() and
    /// blkid_probe_enable_topology() calls.
    /// WARNING: the returned object will be overwritten by the next blkid_probe_get_topology()
    /// call for the same pr. If you want to use more blkid_topopogy objects in the same time you
    /// have to create more blkid_probe handlers (see blkid_new_probe()).
    pub fn get_topology(&self) -> Result<blkid_topology, BlkidError> {
        unsafe { Ok(blkid_probe_get_topology(self.probe)) }
    }
    /// alignment offset in bytes or 0.
    pub fn get_topology_alignment_offset(tp: blkid_topology) -> u64 {
        unsafe { blkid_topology_get_alignment_offset(tp) }
    }
    /// minimum io size in bytes or 0.
    pub fn get_topology_minimum_io_size(tp: blkid_topology) -> u64 {
        unsafe { blkid_topology_get_minimum_io_size(tp) }
    }
    /// optimal io size in bytes or 0.
    pub fn get_topology_optimal_io_size(tp: blkid_topology) -> u64 {
        unsafe { blkid_topology_get_optimal_io_size(tp) }
    }
    /// logical sector size (BLKSSZGET ioctl) in bytes or 0.
    pub fn get_topology_logical_sector_size(tp: blkid_topology) -> u64 {
        unsafe { blkid_topology_get_logical_sector_size(tp) }
    }
    /// logical sector size (BLKSSZGET ioctl) in bytes or 0.
    pub fn get_topology_physical_sector_size(tp: blkid_topology) -> u64 {
        unsafe { blkid_topology_get_physical_sector_size(tp) }
    }

    // pub fn blkid_probe_get_partitions(pr: blkid_probe) -> blkid_partlist;
    // pub fn blkid_partlist_numof_partitions(ls: blkid_partlist)
    // -> ::std::os::raw::c_int;
    // pub fn blkid_partlist_get_table(ls: blkid_partlist) -> blkid_parttable;
    // pub fn blkid_partlist_get_partition(ls: blkid_partlist,
    // n: ::std::os::raw::c_int)
    // -> blkid_partition;
    // pub fn blkid_partlist_devno_to_partition(ls: blkid_partlist, devno: dev_t)
    // -> blkid_partition;
    // pub fn blkid_partition_get_table(par: blkid_partition) -> blkid_parttable;
    // pub fn blkid_partition_get_name(par: blkid_partition)
    // pub fn blkid_partition_get_uuid(par: blkid_partition) -> *const ::std::os::raw::c_char;
    // pub fn blkid_partition_get_partno(par: blkid_partition) -> ::std::os::raw::c_int;
    // pub fn blkid_partition_get_start(par: blkid_partition) -> blkid_loff_t;
    // pub fn blkid_partition_get_size(par: blkid_partition) -> blkid_loff_t;
    // pub fn blkid_partition_get_type(par: blkid_partition) -> ::std::os::raw::c_int;
    // pub fn blkid_partition_get_type_string(par: blkid_partition)
    // -> *const ::std::os::raw::c_char;
    // pub fn blkid_partition_get_flags(par: blkid_partition) -> ::std::os::raw::c_ulonglong;
    // pub fn blkid_partition_is_logical(par: blkid_partition) -> ::std::os::raw::c_int;
    // pub fn blkid_partition_is_extended(par: blkid_partition) -> ::std::os::raw::c_int;
    // pub fn blkid_partition_is_primary(par: blkid_partition) -> ::std::os::raw::c_int;
    // pub fn blkid_parttable_get_type(tab: blkid_parttable) -> *const ::std::os::raw::c_char;
    // pub fn blkid_parttable_get_offset(tab: blkid_parttable) -> blkid_loff_t;
    // pub fn blkid_parttable_get_parent(tab: blkid_parttable) -> blkid_partition;
}

impl Drop for BlkId {
    fn drop(&mut self) {
        if self.probe.is_null() {
            // No cleanup needed
            return;
        }
        unsafe {
            blkid_free_probe(self.probe);
        }
    }
}

// pub fn blkid_put_cache(cache: blkid_cache);
// pub fn blkid_get_cache(cache: *mut blkid_cache,
// filename: *const ::std::os::raw::c_char)
// -> ::std::os::raw::c_int;
// pub fn blkid_dev_devname(dev: blkid_dev) -> *const ::std::os::raw::c_char;
// pub fn blkid_dev_iterate_begin(cache: blkid_cache) -> blkid_dev_iterate;
// pub fn blkid_dev_set_search(iter: blkid_dev_iterate,
// search_type: *mut ::std::os::raw::c_char,
// search_value: *mut ::std::os::raw::c_char)
// -> ::std::os::raw::c_int;
// pub fn blkid_dev_next(iterate: blkid_dev_iterate, dev: *mut blkid_dev)
// -> ::std::os::raw::c_int;
// pub fn blkid_dev_iterate_end(iterate: blkid_dev_iterate);
// pub fn blkid_devno_to_devname(devno: dev_t)
// -> *mut ::std::os::raw::c_char;
// pub fn blkid_devno_to_wholedisk(dev: dev_t,
// diskname: *mut ::std::os::raw::c_char,
// len: usize, diskdevno: *mut dev_t)
// -> ::std::os::raw::c_int;
// pub fn blkid_probe_all(cache: blkid_cache) -> ::std::os::raw::c_int;
// pub fn blkid_probe_all_new(cache: blkid_cache) -> ::std::os::raw::c_int;
// pub fn blkid_probe_all_removable(cache: blkid_cache)
// -> ::std::os::raw::c_int;
// pub fn blkid_get_dev(cache: blkid_cache,
// devname: *const ::std::os::raw::c_char,
// flags: ::std::os::raw::c_int) -> blkid_dev;
// pub fn blkid_get_dev_size(fd: ::std::os::raw::c_int) -> blkid_loff_t;
// pub fn blkid_verify(cache: blkid_cache, dev: blkid_dev) -> blkid_dev;
// pub fn blkid_get_tag_value(cache: blkid_cache,
// tagname: *const ::std::os::raw::c_char,
// devname: *const ::std::os::raw::c_char)
// -> *mut ::std::os::raw::c_char;
// pub fn blkid_get_devname(cache: blkid_cache,
// token: *const ::std::os::raw::c_char,
// value: *const ::std::os::raw::c_char)
// -> *mut ::std::os::raw::c_char;
// pub fn blkid_tag_iterate_begin(dev: blkid_dev) -> blkid_tag_iterate;
// pub fn blkid_tag_next(iterate: blkid_tag_iterate,
// type_: *mut *const ::std::os::raw::c_char,
// value: *mut *const ::std::os::raw::c_char)
// -> ::std::os::raw::c_int;
// pub fn blkid_tag_iterate_end(iterate: blkid_tag_iterate);
// pub fn blkid_dev_has_tag(dev: blkid_dev,
// type_: *const ::std::os::raw::c_char,
// value: *const ::std::os::raw::c_char)
// -> ::std::os::raw::c_int;
// pub fn blkid_find_dev_with_tag(cache: blkid_cache,
// type_: *const ::std::os::raw::c_char,
// value: *const ::std::os::raw::c_char)
// -> blkid_dev;
// pub fn blkid_parse_tag_string(token: *const ::std::os::raw::c_char,
// ret_type: *mut *mut ::std::os::raw::c_char,
// ret_val: *mut *mut ::std::os::raw::c_char)
// -> ::std::os::raw::c_int;
// pub fn blkid_parse_version_string(ver_string:
// const ::std::os::raw::c_char)
// -> ::std::os::raw::c_int;
// pub fn blkid_get_library_version(ver_string:
// mut *const ::std::os::raw::c_char,
// date_string:
// mut *const ::std::os::raw::c_char)
// -> ::std::os::raw::c_int;
// pub fn blkid_encode_string(str: *const ::std::os::raw::c_char,
// str_enc: *mut ::std::os::raw::c_char,
// len: usize) -> ::std::os::raw::c_int;
// pub fn blkid_safe_string(str: *const ::std::os::raw::c_char,
// str_safe: *mut ::std::os::raw::c_char,
// len: usize) -> ::std::os::raw::c_int;
// pub fn blkid_send_uevent(devname: *const ::std::os::raw::c_char,
// action: *const ::std::os::raw::c_char)
// -> ::std::os::raw::c_int;
// pub fn blkid_evaluate_tag(token: *const ::std::os::raw::c_char,
// value: *const ::std::os::raw::c_char,
// cache: *mut blkid_cache)
// -> *mut ::std::os::raw::c_char;
// pub fn blkid_evaluate_spec(spec: *const ::std::os::raw::c_char,
// cache: *mut blkid_cache)
// -> *mut ::std::os::raw::c_char;
// pub fn blkid_new_probe() -> blkid_probe;
// pub fn blkid_reset_probe(pr: blkid_probe);
// pub fn blkid_probe_set_device(pr: blkid_probe, fd: ::std::os::raw::c_int,
// off: blkid_loff_t, size: blkid_loff_t)
// -> ::std::os::raw::c_int;
// pub fn blkid_superblocks_get_name(idx: usize,
// name:
// mut *const ::std::os::raw::c_char,
// usage: *mut ::std::os::raw::c_int)
// -> ::std::os::raw::c_int;
// pub fn blkid_probe_enable_superblocks(pr: blkid_probe,
// enable: ::std::os::raw::c_int)
// -> ::std::os::raw::c_int;
// pub fn blkid_probe_set_superblocks_flags(pr: blkid_probe,
// flags: ::std::os::raw::c_int)
// -> ::std::os::raw::c_int;
// pub fn blkid_probe_reset_superblocks_filter(pr: blkid_probe)
// -> ::std::os::raw::c_int;
// pub fn blkid_probe_invert_superblocks_filter(pr: blkid_probe)
// -> ::std::os::raw::c_int;
// pub fn blkid_probe_filter_superblocks_type(pr: blkid_probe,
// flag: ::std::os::raw::c_int,
// names:
// mut *mut ::std::os::raw::c_char)
// -> ::std::os::raw::c_int;
// pub fn blkid_probe_filter_superblocks_usage(pr: blkid_probe,
// flag: ::std::os::raw::c_int,
// usage: ::std::os::raw::c_int)
// -> ::std::os::raw::c_int;
// pub fn blkid_known_pttype(pttype: *const ::std::os::raw::c_char)
// -> ::std::os::raw::c_int;
// pub fn blkid_probe_enable_partitions(pr: blkid_probe,
// enable: ::std::os::raw::c_int)
// -> ::std::os::raw::c_int;
// pub fn blkid_probe_reset_partitions_filter(pr: blkid_probe)
// -> ::std::os::raw::c_int;
// pub fn blkid_probe_invert_partitions_filter(pr: blkid_probe)
// -> ::std::os::raw::c_int;
// pub fn blkid_probe_filter_partitions_type(pr: blkid_probe,
// flag: ::std::os::raw::c_int,
// names:
// mut *mut ::std::os::raw::c_char)
// -> ::std::os::raw::c_int;
// pub fn blkid_probe_set_partitions_flags(pr: blkid_probe,
// flags: ::std::os::raw::c_int)
// -> ::std::os::raw::c_int;
// pub fn blkid_probe_get_value(pr: blkid_probe,
// num: ::std::os::raw::c_int,
// name: *mut *const ::std::os::raw::c_char,
// data: *mut *const ::std::os::raw::c_char,
// len: *mut usize)
// -> ::std::os::raw::c_int;
