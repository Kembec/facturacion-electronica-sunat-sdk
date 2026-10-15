#!/usr/bin/env python3
"""Independent verifier for the enveloped XML-DSig that sunat-xmldsig produces.

It shares no code with the Rust crate: canonicalization is lxml's (libxml2) and RSA is
`cryptography`'s. It checks the SUNAT profile (inclusive C14N 1.0, RSA-SHA1, one
Reference URI="" with the enveloped transform, SHA1 digest), the digest of the document without
the signature, and the signature over the canonical SignedInfo with the embedded certificate.

Usage: python3 -I verify_independent.py <signed.xml> [<signed.xml> ...]
Exit status 0 when every file verifies; prints one line per file.
"""

import base64
import copy
import hashlib
import sys

from cryptography import x509
from cryptography.hazmat.primitives import hashes
from cryptography.hazmat.primitives.asymmetric import padding
from lxml import etree

DS = "{http://www.w3.org/2000/09/xmldsig#}"
C14N = "http://www.w3.org/TR/2001/REC-xml-c14n-20010315"
RSA_SHA1 = "http://www.w3.org/2000/09/xmldsig#rsa-sha1"
SHA1 = "http://www.w3.org/2000/09/xmldsig#sha1"
ENVELOPED = "http://www.w3.org/2000/09/xmldsig#enveloped-signature"


class VerificationError(Exception):
    pass


def expect(condition, message):
    if not condition:
        raise VerificationError(message)


def b64(element):
    expect(element is not None, "missing base64 element")
    return base64.b64decode("".join((element.text or "").split()))


def first_signature(tree):
    root = tree.getroot()
    if root.tag == DS + "Signature":
        return root
    return root.find(".//" + DS + "Signature")


def remove_keeping_tail(element):
    parent = element.getparent()
    if element.tail:
        previous = element.getprevious()
        if previous is not None:
            previous.tail = (previous.tail or "") + element.tail
        else:
            parent.text = (parent.text or "") + element.tail
    parent.remove(element)


def verify(path):
    parser = etree.XMLParser(resolve_entities=False, no_network=True, load_dtd=False)
    tree = etree.parse(path, parser)
    signature = first_signature(tree)
    expect(signature is not None, "no ds:Signature")

    signed_info = signature.find(DS + "SignedInfo")
    expect(signed_info is not None, "no SignedInfo")
    expect(signed_info.find(DS + "CanonicalizationMethod").get("Algorithm") == C14N, "C14N method")
    expect(signed_info.find(DS + "SignatureMethod").get("Algorithm") == RSA_SHA1, "signature method")
    references = signed_info.findall(DS + "Reference")
    expect(len(references) == 1, "exactly one Reference")
    reference = references[0]
    expect(reference.get("URI") == "", "Reference URI must be empty")
    transforms = [t.get("Algorithm") for t in reference.findall(DS + "Transforms/" + DS + "Transform")]
    expect(transforms == [ENVELOPED], "transforms must be [enveloped]")
    expect(reference.find(DS + "DigestMethod").get("Algorithm") == SHA1, "digest method")
    digest_value = b64(reference.find(DS + "DigestValue"))

    certificate = x509.load_der_x509_certificate(
        b64(signature.find(DS + "KeyInfo/" + DS + "X509Data/" + DS + "X509Certificate"))
    )
    canonical_signed_info = etree.tostring(signed_info, method="c14n", exclusive=False, with_comments=False)
    try:
        certificate.public_key().verify(
            b64(signature.find(DS + "SignatureValue")),
            canonical_signed_info,
            padding.PKCS1v15(),
            hashes.SHA1(),
        )
    except Exception as exc:  # cryptography raises InvalidSignature
        raise VerificationError("SignatureValue does not verify") from exc

    unsigned = copy.deepcopy(tree)
    remove_keeping_tail(first_signature(unsigned))
    canonical_document = etree.tostring(unsigned, method="c14n", exclusive=False, with_comments=False)
    expect(hashlib.sha1(canonical_document).digest() == digest_value, "DigestValue does not match")
    return base64.b64encode(digest_value).decode()


def main(paths):
    failed = False
    for path in paths:
        try:
            digest = verify(path)
            print(f"OK {path} digest={digest}")
        except (VerificationError, etree.XMLSyntaxError, ValueError) as exc:
            failed = True
            print(f"FAIL {path}: {exc}")
    return 1 if failed or not paths else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
