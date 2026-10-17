using System;
using System.Runtime.InteropServices;

namespace Kembec.SunatSdk.Interop
{
    /// <summary>
    /// The six functions of the C ABI (bindings/ffi/include/sunat_sdk.h). <c>StdCall</c> is the
    /// <c>SUNAT_SDK_CALL</c> of 32-bit Windows and is ignored everywhere else.
    /// </summary>
    internal static class NativeMethods
    {
        /// <summary>Probed as sunat_sdk_ffi.dll, libsunat_sdk_ffi.so or libsunat_sdk_ffi.dylib.</summary>
        internal const string Library = "sunat_sdk_ffi";

        internal const int StatusOk = 0;
        internal const int StatusError = 1;
        internal const uint AbiVersion = 1;

        [DllImport(Library, CallingConvention = CallingConvention.StdCall, ExactSpelling = true)]
        internal static extern int sunat_sdk_call(byte[] request, out IntPtr response);

        [DllImport(Library, CallingConvention = CallingConvention.StdCall, ExactSpelling = true)]
        internal static extern void sunat_sdk_free(IntPtr response);

        [DllImport(Library, CallingConvention = CallingConvention.StdCall, ExactSpelling = true)]
        internal static extern UIntPtr sunat_sdk_response_len(IntPtr response);

        [DllImport(Library, CallingConvention = CallingConvention.StdCall, ExactSpelling = true)]
        internal static extern IntPtr sunat_sdk_version();

        [DllImport(Library, CallingConvention = CallingConvention.StdCall, ExactSpelling = true)]
        internal static extern uint sunat_sdk_abi_version();
    }
}
