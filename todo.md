1. [ ] Add brs,extended ID, rtr flags etc. to CANframe. Reorganize like canfd_frame in c. Provide is_extended() funcs
2. [ ] add make_can_frame() function. Or new(). Always having to create a make_can_frame function\
3. [ ] handle malformed inputs. Identify input souces (reading only??), check inputs & handle errors.
4. [ ] remove uses of anyhow! as they create a new error, discarding the old one. Prefer .with_context. Double check how this works.
5. [ ] consider change CanLogParser iterator from `type Item = CanFrame` to `type Item=anyhow::Result<CanFrame>`. Matches BufRead::lines() so seems idiomatic. As-is, hides any errors
