Test material.

- `send_bill_envelope.xml` and `get_status_cdr_envelope.xml` are the request examples of SUNAT's
  programmer's manual (SEE v2.1, sendBill p. 17 and getStatusCdr p. 22) with the public beta
  credentials (`MODDATOS`), the `Type` attribute (PasswordText) on the WS-Security password, and an
  inline base64 zip in place of the manual's `cid:` reference. Line breaks are layout only: tests
  compare without them.
- `R-20150147718-01-FA01-981.xml` and `R-20150147718-01-FT01-982.zip` are synthetic CDRs written for
  these tests after the ApplicationResponse examples of the same manual (annex C.1, accepted with
  notes 4031 and 4001; annex C.2, rejected with code 2047), ISO-8859-1 and without SUNAT's
  signature. The zip holds an empty `dummy/` folder first and then the XML, as the manual describes
  SUNAT's zips.
