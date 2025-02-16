## GateConvert

The library allows to easily convert Gate circuit from/to one of few foreign formats.
This library is used by `gateconvert_exec` program that allow conversion by command line
interface.

A conversion to foreign logic format writes result data into output (by `Write` trait).
A conversion from foreign logic format returns Gate circuit object and sometimes
additional mapping. Any functions that make conversion returns Result to allow handle
various errors.
