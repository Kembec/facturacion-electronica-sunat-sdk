using System;
using System.Collections.Generic;
using System.IO;
using System.Runtime.InteropServices;

namespace Kembec.SunatSdk.Interop
{
    /// <summary>
    /// .NET Framework ignores runtimes/&lt;rid&gt;/native, so the package's targets copy both Windows builds
    /// under the application and this loads the one of the process's bitness before the first call; the
    /// DllImports then bind to the module already loaded under that name. .NET resolves the native
    /// asset from the application's deps.json and needs nothing here.
    /// </summary>
    internal static class NativeLibraryLoader
    {
        internal static void Preload()
        {
#if NETFRAMEWORK
            var rid = IntPtr.Size == 8 ? "win-x64" : "win-x86";
            foreach (var directory in Candidates())
            {
                var path = Path.Combine(directory, "runtimes", rid, "native", "sunat_sdk_ffi.dll");
                if (File.Exists(path) && LoadLibraryW(path) != IntPtr.Zero)
                {
                    return;
                }
            }
#endif
        }

#if NETFRAMEWORK
        /// <summary>
        /// The application directory, its private bin path (ASP.NET: the site root plus <c>bin</c>) and the
        /// directory this assembly was copied from (its original location when ASP.NET shadow-copies it).
        /// </summary>
        private static IEnumerable<string> Candidates()
        {
            var domain = AppDomain.CurrentDomain;
            yield return domain.BaseDirectory;
            if (!string.IsNullOrEmpty(domain.RelativeSearchPath))
            {
                yield return Path.Combine(domain.BaseDirectory, domain.RelativeSearchPath);
            }
            var codeBase = typeof(NativeLibraryLoader).Assembly.CodeBase;
            if (Uri.TryCreate(codeBase, UriKind.Absolute, out var uri) && uri.IsFile)
            {
                var directory = Path.GetDirectoryName(uri.LocalPath);
                if (directory != null)
                {
                    yield return directory;
                }
            }
        }

        [DllImport("kernel32", CharSet = CharSet.Unicode, ExactSpelling = true, SetLastError = true)]
        private static extern IntPtr LoadLibraryW(string path);
#endif
    }
}
