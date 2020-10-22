This is a tiny crate with one purpose:
generate temporary filenames
from a hash of arbitrary data
so that they are consistent across different crates.
Its intended purpose is to either
	- give a consistent name, which can -- for example -- be used to cache the expensive output of an input across runs,
	- or to allow a caller to know what filename a different crate will have given some abstract data.
It would be trivially easy to replicate; it exists primarily to allow consistency across
otherwise unrelated crates when handling data
