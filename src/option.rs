#[derive(Debug)]
pub(crate) enum CidAllocMode {
    Linear,
    Bitmap,
}

#[derive(Debug)]
pub(crate) struct Opt {
    pub(crate) nodatacache: bool,
    pub(crate) cidalloc: CidAllocMode,
    #[allow(dead_code)]
    pub(crate) debug: bool,
}

impl Opt {
    fn newopt() -> getopts::Options {
        let mut gopt = getopts::Options::new();
        gopt.optflag("", "nodatacache", "");
        gopt.optopt("", "cidalloc", "", "<linear|bitmap>");
        gopt.optflag("h", "help", "");
        gopt.optflag("", "debug", "");
        gopt
    }

    pub(crate) fn new(args: &[&str]) -> nix::Result<Self> {
        let gopt = Opt::newopt();
        let matches = match gopt.parse(args) {
            Ok(v) => v,
            Err(e) => {
                log::error!("{e}");
                return Err(nix::errno::Errno::EINVAL);
            }
        };
        if matches.opt_present("h") {
            println!("{}", gopt.usage("HAMMER2 options"));
            return Err(nix::errno::Errno::UnknownErrno); // 0
        }
        let nodatacache = matches.opt_present("nodatacache");
        let cidalloc = match matches.opt_str("cidalloc") {
            Some(v) => match v.as_str() {
                "linear" => CidAllocMode::Linear,
                "bitmap" => CidAllocMode::Bitmap,
                _ => return Err(nix::errno::Errno::EINVAL),
            },
            None => CidAllocMode::Linear,
        };
        let debug = matches.opt_present("debug");
        Ok(Self {
            nodatacache,
            cidalloc,
            debug,
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_opt_nodatacache() {
        match super::Opt::new(&["--nodatacache"]) {
            Ok(v) => assert!(v.nodatacache),
            Err(e) => panic!("{e}"),
        }
        match super::Opt::new(&[]) {
            Ok(v) => assert!(!v.nodatacache),
            Err(e) => panic!("{e}"),
        }
    }

    #[test]
    fn test_opt_cidalloc() {
        match super::Opt::new(&["--cidalloc", "linear"]) {
            Ok(v) => match v.cidalloc {
                super::CidAllocMode::Linear => (),
                v @ super::CidAllocMode::Bitmap => panic!("{v:?}"),
            },
            Err(e) => panic!("{e}"),
        }
        match super::Opt::new(&["--cidalloc", "bitmap"]) {
            Ok(v) => match v.cidalloc {
                super::CidAllocMode::Bitmap => (),
                v @ super::CidAllocMode::Linear => panic!("{v:?}"),
            },
            Err(e) => panic!("{e}"),
        }
        match super::Opt::new(&["--cidalloc", "xxx"]) {
            Ok(v) => panic!("{v:?}"),
            Err(nix::errno::Errno::EINVAL) => (),
            Err(e) => panic!("{e}"),
        }
    }

    #[test]
    fn test_opt_help() {
        match super::Opt::new(&["-h"]) {
            Ok(v) => panic!("{v:?}"),
            Err(nix::errno::Errno::UnknownErrno) => (),
            Err(e) => panic!("{e}"),
        }
        match super::Opt::new(&["--h"]) {
            Ok(v) => panic!("{v:?}"),
            Err(nix::errno::Errno::UnknownErrno) => (),
            Err(e) => panic!("{e}"),
        }
    }

    #[test]
    fn test_opt_debug() {
        match super::Opt::new(&["--debug"]) {
            Ok(v) => assert!(v.debug),
            Err(e) => panic!("{e}"),
        }
        match super::Opt::new(&[]) {
            Ok(v) => assert!(!v.debug),
            Err(e) => panic!("{e}"),
        }
    }
}
