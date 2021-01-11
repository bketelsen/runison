# runison 
A tool to synchronize directories between two networked computers

## server
```
RUST_BACKTRACE=1 ./target/debug/runison -d -c /home/bjk/src/github.com/bketelsen/runison/test/server.toml server
```
## client
```
RUST_BACKTRACE=1 ./target/debug/runison -d -c /home/bjk/src/github.com/bketelsen/runison/test/client.toml client -n ghanima
```

## Status

- [x] Create archive of before state of sync directory
- [ ] everything else

*** This application has no tests whatsoever and shouldn't even be allowed on the same computer as your important data***
Don't use this yet. Please. Just don't.

## Current Working Plan

Roughly follow Unison's methodology as described in the [User Documentation](https://www.cis.upenn.edu/~bcpierce/unison/download/releases/stable/unison-manual.html#recon)

Unison on the client invokes the `unison` binary on the target host. `runison` will assume that it is running as a daemon on the target host.

quote from Unison docs:

### Unison operates in several distinct stages:
* On each host, it compares its archive file (which records the state of each path in the replica when it was last synchronized) with the current contents of the replica, to determine which paths have been updated.
* It checks for “false conflicts” — paths that have been updated on both replicas, but whose current values are identical. These paths are silently marked as synchronized in the archive files in both replicas.
* It displays all the updated paths to the user. For updates that do not conflict, it suggests a default action (propagating the new contents from the updated replica to the other). Conflicting updates are just displayed. The user is given an opportunity to examine the current state of affairs, change the default actions for nonconflicting updates, and choose actions for conflicting updates.
* It performs the selected actions, one at a time. Each action is performed by first transferring the new contents to a temporary file on the receiving host, then atomically moving them into place.
* It updates its archive files to reflect the new state of the replicas.

## Notes & Todos

* Looks like [if/let](https://doc.rust-lang.org/stable/rust-by-example/flow_control/if_let.html) is a nicer way to do some of the things I'm currently doing with match/unwrap. Investigate more.
```
    // If you need to specify a failure, use an else:
    if let Some(i) = letter {
        println!("Matched {:?}!", i);
    } else {
        // Destructure failed. Change to the failure case.
        println!("Didn't match a number. Let's go with a letter!");
    }
```
