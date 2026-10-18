//! Guest data of a lodging (0202) or tour package (0205) export line: one `cac:AdditionalItemProperty`
//! per field, in catalog 55 order (4000 to 4009). `Name` is the catalog description (4235 if empty);
//! the dates go in `UsabilityPeriod/StartDate` (3065) and the days in `DurationMeasure` (3135).

use sunat_model::Lodging;

use crate::writer::XmlWriter;

enum Kind {
    Value,
    Date,
    Days,
}

type Field = fn(&Lodging) -> Option<&String>;

const PROPERTIES: &[(&str, &str, Kind, Field)] = &[
    ("4000", "Beneficio Hospedajes-Paquete turístico: Código de país de emisión del pasaporte", Kind::Value, |l| l.passport_country.as_ref()),
    ("4001", "Beneficio Hospedajes: Código de país de residencia del sujeto no domiciliado", Kind::Value, |l| l.residence_country.as_ref()),
    ("4002", "Beneficio Hospedajes: Fecha de ingreso al país", Kind::Date, |l| l.country_entry_date.as_ref()),
    ("4003", "Beneficio Hospedajes: Fecha de Ingreso al Establecimiento", Kind::Date, |l| l.check_in_date.as_ref()),
    ("4004", "Beneficio Hospedajes: Fecha de Salida del Establecimiento", Kind::Date, |l| l.check_out_date.as_ref()),
    ("4005", "Beneficio Hospedajes: Número de Días de Permanencia", Kind::Days, |l| l.stay_days.as_ref()),
    ("4006", "Beneficio Hospedajes: Fecha de Consumo", Kind::Date, |l| l.consumption_date.as_ref()),
    ("4007", "Beneficio Hospedajes-Paquete turístico: Nombres y apellidos del huesped", Kind::Value, |l| l.guest_name.as_ref()),
    ("4008", "Beneficio Hospedajes-Paquete turístico: Tipo de documento de identidad del huesped", Kind::Value, |l| l.guest_document_type.as_ref()),
    ("4009", "Beneficio Hospedajes-Paquete turístico: Número de documento de identidad del huesped", Kind::Value, |l| l.guest_document_number.as_ref()),
];

pub(crate) fn properties(w: &mut XmlWriter, lodging: &Lodging) {
    for (code, name, kind, get) in PROPERTIES {
        let Some(value) = get(lodging) else { continue };
        w.open("cac:AdditionalItemProperty", &[]);
        w.text("cbc:Name", name);
        w.text("cbc:NameCode", code);
        match kind {
            Kind::Value => w.text("cbc:Value", value),
            Kind::Date => period(w, |w| w.text("cbc:StartDate", value)),
            Kind::Days => period(w, |w| w.leaf("cbc:DurationMeasure", &[("unitCode", "DAY")], value)),
        }
        w.close();
    }
}

fn period(w: &mut XmlWriter, body: impl FnOnce(&mut XmlWriter)) {
    w.open("cac:UsabilityPeriod", &[]);
    body(w);
    w.close();
}
