using System;
using System.Collections.Generic;
using System.Text.Json;

namespace Kembec.SunatSdk.Results
{
    /// <summary>The response does not have the shape docs/cli.md promises for its command.</summary>
    internal sealed class ResponseShapeException : Exception
    {
        internal ResponseShapeException(string message)
            : base(message)
        {
        }
    }

    /// <summary>Typed reads of a response object; a missing or mistyped required field is a shape error.</summary>
    internal static class Fields
    {
        internal static string Text(JsonElement value, string name)
        {
            return OptionalText(value, name) ?? throw Missing(name, "a string");
        }

        internal static string? OptionalText(JsonElement value, string name)
        {
            if (!TryGet(value, name, out var field))
            {
                return null;
            }
            return field.ValueKind == JsonValueKind.String ? field.GetString() : throw Missing(name, "a string or null");
        }

        internal static int? OptionalInt(JsonElement value, string name)
        {
            if (!TryGet(value, name, out var field))
            {
                return null;
            }
            return field.ValueKind == JsonValueKind.Number && field.TryGetInt32(out var number) ? number : throw Missing(name, "a whole number or null");
        }

        internal static int Int(JsonElement value, string name)
        {
            return OptionalInt(value, name) ?? throw Missing(name, "a whole number");
        }

        internal static bool Bool(JsonElement value, string name)
        {
            if (TryGet(value, name, out var field) && (field.ValueKind == JsonValueKind.True || field.ValueKind == JsonValueKind.False))
            {
                return field.GetBoolean();
            }
            throw Missing(name, "a boolean");
        }

        /// <summary>A list of strings; absent or null is empty.</summary>
        internal static IReadOnlyList<string> Texts(JsonElement value, string name)
        {
            var list = new List<string>();
            if (!TryGet(value, name, out var field))
            {
                return list;
            }
            if (field.ValueKind != JsonValueKind.Array)
            {
                throw Missing(name, "a list of strings");
            }
            foreach (var item in field.EnumerateArray())
            {
                list.Add(item.ValueKind == JsonValueKind.String ? item.GetString()! : throw Missing(name, "a list of strings"));
            }
            return list;
        }

        internal static byte[]? OptionalBase64(JsonElement value, string name)
        {
            var text = OptionalText(value, name);
            if (text == null)
            {
                return null;
            }
            try
            {
                return Convert.FromBase64String(text);
            }
            catch (FormatException)
            {
                throw Missing(name, "base64");
            }
        }

        internal static JsonElement? OptionalObject(JsonElement value, string name)
        {
            if (!TryGet(value, name, out var field))
            {
                return null;
            }
            return field.ValueKind == JsonValueKind.Object ? field : throw Missing(name, "an object or null");
        }

        internal static Outcome ReadOutcome(JsonElement value, params Outcome[] allowed)
        {
            var name = Text(value, "outcome");
            foreach (var outcome in allowed)
            {
                if (OutcomeNames.Of(outcome) == name)
                {
                    return outcome;
                }
            }
            throw new ResponseShapeException("outcome `" + name + "` is not one this command gives");
        }

        /// <summary>Present and not null.</summary>
        private static bool TryGet(JsonElement value, string name, out JsonElement field)
        {
            if (value.ValueKind != JsonValueKind.Object)
            {
                throw new ResponseShapeException("expected an object");
            }
            return value.TryGetProperty(name, out field) && field.ValueKind != JsonValueKind.Null;
        }

        private static ResponseShapeException Missing(string name, string expected)
        {
            return new ResponseShapeException("`" + name + "` is not " + expected);
        }
    }
}
