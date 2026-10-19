# OASIS UBL 2.1 schemas

`ubl-2.1/` holds the XML schemas of OASIS Universal Business Language (UBL) 2.1, OS release of
4 November 2013 (<http://docs.oasis-open.org/ubl/os-UBL-2.1/>): `maindoc/UBL-DespatchAdvice-2.1.xsd`,
`maindoc/UBL-Invoice-2.1.xsd` and the `common/` modules they import. They are unmodified apart from what the OASIS package ships; the
tests use them to validate the XML this SDK generates (`crates/sunat-ubl/tests/xsd.rs`). They are not
part of the SDK's published crates.

Copyright (c) OASIS Open 2013. All Rights Reserved.

Under the OASIS notices for UBL 2.1, the specification and its schemas may be copied, shared and used
to make derivative works that explain or help implement it, published without restriction provided
that this copyright notice and the OASIS notices stay on every copy. They are provided "AS IS", without
warranty of any kind. "OASIS" is a trademark of OASIS Open. The full notices are in the
"Notices" section of <http://docs.oasis-open.org/ubl/os-UBL-2.1/UBL-2.1.html>.
