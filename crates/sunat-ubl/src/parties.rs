//! Signature reference, issuer (`AccountingSupplierParty`) and customer
//! (`AccountingCustomerParty`), per sections «Firma digital», «Datos del emisor» and «Datos del
//! adquirente o usuario» of SUNAT's UBL 2.1 guides.

use sunat_model::{Address, Client, Company};

use crate::writer::XmlWriter;
use crate::Options;

/// Catalog 06 code of a RUC.
pub(crate) const RUC_SCHEME: &str = "6";

/// `cac:Signature`: who signs and where the `ds:Signature` is (`#` + its `Id`).
pub(crate) fn signature_reference(w: &mut XmlWriter, issuer: &Company, options: &Options) {
    w.open("cac:Signature", &[]);
    w.text("cbc:ID", &options.signature_reference_id);
    w.open("cac:SignatoryParty", &[]);
    w.open("cac:PartyIdentification", &[]);
    w.text("cbc:ID", &issuer.ruc);
    w.close();
    w.open("cac:PartyName", &[]);
    w.text("cbc:Name", &issuer.legal_name);
    w.close();
    w.close();
    w.open("cac:DigitalSignatureAttachment", &[]);
    w.open("cac:ExternalReference", &[]);
    w.text("cbc:URI", &format!("#{}", options.signature_id));
    w.close();
    w.close();
    w.close();
}

pub(crate) fn supplier(w: &mut XmlWriter, issuer: &Company) {
    w.open("cac:AccountingSupplierParty", &[]);
    w.open("cac:Party", &[]);
    party_identification(w, RUC_SCHEME, &issuer.ruc);
    if let Some(trade_name) = non_empty(&issuer.trade_name) {
        w.open("cac:PartyName", &[]);
        w.text("cbc:Name", trade_name);
        w.close();
    }
    w.open("cac:PartyLegalEntity", &[]);
    w.text("cbc:RegistrationName", &issuer.legal_name);
    issuer_address(w, &issuer.address);
    w.close();
    contact(w, &issuer.telephone, &issuer.email);
    w.close();
    w.close();
}

pub(crate) fn customer(w: &mut XmlWriter, customer: &Client) {
    w.open("cac:AccountingCustomerParty", &[]);
    w.open("cac:Party", &[]);
    party_identification(w, &customer.document_type, &customer.document_number);
    w.open("cac:PartyLegalEntity", &[]);
    w.text("cbc:RegistrationName", &customer.legal_name);
    if let Some(address) = &customer.address {
        customer_address(w, address);
    }
    w.close();
    contact(w, &customer.telephone, &customer.email);
    w.close();
    w.close();
}

fn party_identification(w: &mut XmlWriter, scheme: &str, number: &str) {
    w.open("cac:PartyIdentification", &[]);
    w.leaf("cbc:ID", &[("schemeID", scheme)], number);
    w.close();
}

/// Fiscal address of the issuer: ubigeo, establishment code (catalog «código de domicilio
/// fiscal o de local anexo»), urbanization, province, department, district, street, country.
fn issuer_address(w: &mut XmlWriter, address: &Address) {
    w.open("cac:RegistrationAddress", &[]);
    w.text_opt("cbc:ID", address.ubigeo.as_deref());
    w.text_opt("cbc:AddressTypeCode", address.establishment_code.as_deref());
    w.text_opt("cbc:CitySubdivisionName", address.urbanization.as_deref());
    w.text_opt("cbc:CityName", address.province.as_deref());
    w.text_opt("cbc:CountrySubentity", address.department.as_deref());
    w.text_opt("cbc:District", address.district.as_deref());
    address_line_and_country(w, address);
    w.close();
}

/// Customer address: ubigeo when known, the street and the country.
fn customer_address(w: &mut XmlWriter, address: &Address) {
    w.open("cac:RegistrationAddress", &[]);
    w.text_opt("cbc:ID", address.ubigeo.as_deref());
    address_line_and_country(w, address);
    w.close();
}

fn address_line_and_country(w: &mut XmlWriter, address: &Address) {
    if let Some(street) = non_empty(&address.street) {
        w.open("cac:AddressLine", &[]);
        w.text("cbc:Line", street);
        w.close();
    }
    if let Some(country) = non_empty(&address.country_code) {
        w.open("cac:Country", &[]);
        w.text("cbc:IdentificationCode", country);
        w.close();
    }
}

fn contact(w: &mut XmlWriter, telephone: &Option<String>, email: &Option<String>) {
    let (telephone, email) = (non_empty(telephone), non_empty(email));
    if telephone.is_none() && email.is_none() {
        return;
    }
    w.open("cac:Contact", &[]);
    w.text_opt("cbc:Telephone", telephone);
    w.text_opt("cbc:ElectronicMail", email);
    w.close();
}

fn non_empty(value: &Option<String>) -> Option<&str> {
    value.as_deref().filter(|v| !v.is_empty())
}
