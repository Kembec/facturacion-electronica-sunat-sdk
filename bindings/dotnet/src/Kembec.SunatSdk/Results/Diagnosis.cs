using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace Kembec.SunatSdk
{
    /// <summary>A code SUNAT answered, with the input fields that may have caused it (the <c>diagnosis</c> of docs/cli.md).</summary>
    public sealed class Diagnosis
    {
        internal Diagnosis(JsonElement value)
        {
            Code = Results.Fields.Text(value, "code");
            var detail = Results.Fields.OptionalObject(value, "detail");
            Detail = detail.HasValue ? new DiagnosisDetail(detail.Value) : null;
            Fields = Results.Fields.Objects(value, "fields").Select(field => new DiagnosisField(field)).ToList();
        }

        /// <summary>The code SUNAT answered (<c>2800</c>, <c>4287</c>…).</summary>
        public string Code { get; }

        /// <summary>What SUNAT's message names (node and value), or null when it names none.</summary>
        public DiagnosisDetail? Detail { get; }

        /// <summary>
        /// The candidate fields; empty for a code the rules do not list, and for summaries, void communications and
        /// tickets. With a <see cref="Detail"/>, only those on its node, unless none is.
        /// </summary>
        public IReadOnlyList<DiagnosisField> Fields { get; }

        /// <summary>The <c>diagnosis</c> of an outcome; absent or null is empty.</summary>
        internal static IReadOnlyList<Diagnosis> ListFrom(JsonElement value)
        {
            return Results.Fields.Objects(value, "diagnosis").Select(item => new Diagnosis(item)).ToList();
        }
    }

    /// <summary>
    /// What SUNAT's message names as <c>(nodo: "&lt;node&gt;" valor: "&lt;value&gt;")</c>, e.g. <c>cbc:ID/schemeID</c> = <c>1</c>.
    /// The form is observed in beta answers; SUNAT does not document it.
    /// </summary>
    public sealed class DiagnosisDetail
    {
        internal DiagnosisDetail(JsonElement value)
        {
            Node = Results.Fields.Text(value, "node");
            Value = Results.Fields.Text(value, "value");
        }

        /// <summary>The last element and, if any, its attribute, as SUNAT writes it.</summary>
        public string Node { get; }

        /// <summary>The value SUNAT read in that node.</summary>
        public string Value { get; }
    }

    /// <summary>One candidate field for a code, from SUNAT's validation rules of 26.08.2026 (docs/cli.md#diagnosis).</summary>
    public sealed class DiagnosisField
    {
        /// <summary><see cref="Certainty"/> when the node is the rule's own and one field writes it.</summary>
        public const string Source = "fuente";

        /// <summary><see cref="Certainty"/> when the rule computes over several nodes.</summary>
        public const string Inferred = "inferido";

        internal DiagnosisField(JsonElement value)
        {
            InputPath = Results.Fields.OptionalText(value, "input_path");
            Xpath = Results.Fields.OptionalText(value, "xpath");
            Certainty = Results.Fields.Text(value, "certainty");
        }

        /// <summary>
        /// A field of the input model (<c>customer.document_type</c>, <c>lines[].unit_price</c>), <c>sdk</c> when the
        /// SDK writes the node itself, or null when the input model has no such field.
        /// </summary>
        public string? InputPath { get; }

        /// <summary>The node the rule checks; null for rules on the file, the signature or the service.</summary>
        public string? Xpath { get; }

        /// <summary><see cref="Source"/> or <see cref="Inferred"/>.</summary>
        public string Certainty { get; }
    }
}
