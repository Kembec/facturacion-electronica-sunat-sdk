//! `sac:SUNATPerceptionSummaryDocumentReference` of a daily summary line: after `cac:BillingReference` and before
//! `cac:Status` (Resumen Diario sheet, rows 58-75; Greenter's `summary.xml.twig` in the same place). The model
//! already checked the regime, the amounts and that they are in PEN (2517, 2608, 2685, 2893-2897, 2986).

use sunat_model::SummaryPerception;

use crate::amount::two_decimals;
use crate::writer::XmlWriter;

/// The perception is always reported in PEN (2685, 2690).
const PEN: [(&str, &str); 1] = [("currencyID", "PEN")];

pub(crate) fn perception(w: &mut XmlWriter, perception: &SummaryPerception) {
    w.open("sac:SUNATPerceptionSummaryDocumentReference", &[]);
    w.text("sac:SUNATPerceptionSystemCode", &perception.system_code);
    // The model rejects any code outside catalog 22 (2517), so the rate is always there.
    let percent = sunat_model::perception_percent(&perception.system_code).unwrap_or_default();
    w.text("sac:SUNATPerceptionPercent", &percent);
    w.leaf("cbc:TotalInvoiceAmount", &PEN, &two_decimals(&perception.amount));
    w.leaf("sac:SUNATTotalCashed", &PEN, &two_decimals(&perception.total_cashed));
    w.leaf("cbc:TaxableAmount", &PEN, &two_decimals(&perception.base));
    w.close();
}
