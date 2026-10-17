using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Encodings.Web;
using System.Text.Json;
using Kembec.SunatSdk.Results;

namespace Kembec.SunatSdk.Interop
{
    /// <summary>A flag of the request: <c>"timeout": 30</c> is the CLI's <c>--timeout 30</c>.</summary>
    internal readonly struct Flag
    {
        internal Flag(string name, string text)
        {
            Name = name;
            Text = text ?? throw new ArgumentNullException(name);
            Number = 0;
        }

        internal Flag(string name, int number)
        {
            Name = name;
            Text = null;
            Number = number;
        }

        internal string Name { get; }

        internal string? Text { get; }

        internal int Number { get; }
    }

    /// <summary>One call of the C ABI: the request in, the response read, released and mapped.</summary>
    internal static class Native
    {
        private static readonly JsonWriterOptions WriterOptions = new JsonWriterOptions
        {
            // The request goes to the library, not to an HTML page: XML stays readable and short.
            Encoder = JavaScriptEncoder.UnsafeRelaxedJsonEscaping,
        };

        private static readonly UTF8Encoding StrictUtf8 = new UTF8Encoding(false, true);
        private static readonly object Gate = new object();
        private static volatile bool _ready;
        private static string? _version;

        internal static string Version
        {
            get
            {
                EnsureLoaded();
                return _version ??= ReadStatic(NativeMethods.sunat_sdk_version());
            }
        }

        /// <summary>
        /// Runs <paramref name="command"/> and maps its response with <paramref name="read"/>. Status 1 is a
        /// <see cref="SunatSdkException"/> with the library's message; a response that is not the shape the
        /// contract promises is one too.
        /// </summary>
        internal static T Run<T>(string command, Action<Utf8JsonWriter> writeInput, IReadOnlyList<Flag> flags, Func<JsonElement, string, T> read)
        {
            EnsureLoaded();
            int status;
            string text;
            using (var request = new ClearingBuffer(4096))
            {
                Write(request, command, writeInput, flags);
                status = NativeMethods.sunat_sdk_call(request.Array, out var response);
                text = Take(command, response);
            }
            try
            {
                using (var document = JsonDocument.Parse(text))
                {
                    var root = document.RootElement;
                    if (status == NativeMethods.StatusError)
                    {
                        throw new SunatSdkException(command, Fields.Text(root, "error"));
                    }
                    if (status != NativeMethods.StatusOk)
                    {
                        throw new SunatSdkException(command, "the library answered status " + status);
                    }
                    return read(root, text);
                }
            }
            catch (JsonException e)
            {
                throw new SunatSdkException(command, "the library's response is not JSON", e);
            }
            catch (ResponseShapeException e)
            {
                throw new SunatSdkException(command, "unexpected response: " + e.Message, e);
            }
        }

        internal static void EnsureLoaded()
        {
            if (_ready)
            {
                return;
            }
            lock (Gate)
            {
                if (_ready)
                {
                    return;
                }
                NativeLibraryLoader.Preload();
                uint abi;
                try
                {
                    abi = NativeMethods.sunat_sdk_abi_version();
                }
                catch (DllNotFoundException e)
                {
                    throw new SunatSdkLoadException(NotFound(), e);
                }
                catch (BadImageFormatException e)
                {
                    throw new SunatSdkLoadException(NotFound() + " A library of another bitness was found first.", e);
                }
                catch (EntryPointNotFoundException e)
                {
                    throw new SunatSdkLoadException("the loaded " + NativeMethods.Library + " is not the SDK's C ABI", e);
                }
                if (abi != NativeMethods.AbiVersion)
                {
                    throw new SunatSdkLoadException(
                        "the native library speaks ABI " + abi + " and this package ABI " + NativeMethods.AbiVersion
                        + ": the package and its native library come from different versions");
                }
                _ready = true;
            }
        }

        private static void Write(ClearingBuffer request, string command, Action<Utf8JsonWriter> writeInput, IReadOnlyList<Flag> flags)
        {
            using (var writer = new Utf8JsonWriter(request, WriterOptions))
            {
                writer.WriteStartObject();
                writer.WriteString("command", command);
                // The input (an XML of megabytes, at most) goes before the flags, so the buffer has done its
                // growing before a password is written into it.
                writer.WritePropertyName("input");
                writeInput(writer);
                if (flags.Count > 0)
                {
                    writer.WriteStartObject("flags");
                    foreach (var flag in flags)
                    {
                        if (flag.Text != null)
                        {
                            writer.WriteString(flag.Name, flag.Text);
                        }
                        else
                        {
                            writer.WriteNumber(flag.Name, flag.Number);
                        }
                    }
                    writer.WriteEndObject();
                }
                writer.WriteEndObject();
            }
            request.GetSpan(1)[0] = 0;
            request.Advance(1);
        }

        private static string Take(string command, IntPtr response)
        {
            if (response == IntPtr.Zero)
            {
                throw new SunatSdkException(command, "the library stored no response");
            }
            try
            {
                var length = NativeMethods.sunat_sdk_response_len(response).ToUInt64();
                if (length > int.MaxValue)
                {
                    throw new SunatSdkException(command, "the library's response is larger than 2 GB");
                }
                var bytes = new byte[(int)length];
                Marshal.Copy(response, bytes, 0, bytes.Length);
                return StrictUtf8.GetString(bytes);
            }
            finally
            {
                NativeMethods.sunat_sdk_free(response);
            }
        }

        private static string ReadStatic(IntPtr text)
        {
            var length = 0;
            while (Marshal.ReadByte(text, length) != 0)
            {
                length++;
            }
            var bytes = new byte[length];
            Marshal.Copy(text, bytes, 0, length);
            return StrictUtf8.GetString(bytes);
        }

        private static string NotFound()
        {
            return "the native library " + NativeMethods.Library + " was not found for this process ("
                + Environment.OSVersion + ", " + (IntPtr.Size * 8) + "-bit). The package carries win-x86, win-x64, "
                + "linux-x64, linux-arm64, osx-x64 and osx-arm64; on .NET Framework its runtimes folder must be "
                + "next to the application.";
        }
    }
}
