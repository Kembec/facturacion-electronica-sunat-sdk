using System;

namespace Kembec.SunatSdk
{
    /// <summary>
    /// A command that did not run: the SDK's own error (the CLI's <c>{"error"}</c> with exit code 1, such as
    /// an invalid document, a missing certificate or a send without an answer) or a response that is not the
    /// contract's. An answer from SUNAT, a rejection included, is a result, never this exception.
    /// </summary>
    public class SunatSdkException : Exception
    {
        /// <summary>Creates the exception for <paramref name="command"/>.</summary>
        public SunatSdkException(string? command, string message)
            : base(message)
        {
            Command = command;
        }

        /// <summary>Creates the exception for <paramref name="command"/> with its cause.</summary>
        public SunatSdkException(string? command, string message, Exception innerException)
            : base(message, innerException)
        {
            Command = command;
        }

        /// <summary>The command of docs/cli.md that failed (<c>build</c>, <c>send</c>…); null if none ran.</summary>
        public string? Command { get; }
    }

    /// <summary>
    /// The native library was not found, has another bitness than the process, or comes from a version of the
    /// SDK with another ABI. Nothing ran.
    /// </summary>
    public sealed class SunatSdkLoadException : SunatSdkException
    {
        /// <summary>Creates the exception.</summary>
        public SunatSdkLoadException(string message)
            : base(null, message)
        {
        }

        /// <summary>Creates the exception with its cause.</summary>
        public SunatSdkLoadException(string message, Exception innerException)
            : base(null, message, innerException)
        {
        }
    }
}
