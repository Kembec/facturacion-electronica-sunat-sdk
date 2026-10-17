using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Net;
using System.Net.Sockets;
using System.Reflection;
using System.Security.Cryptography.Xml;
using System.Text;
using System.Text.Json;
using System.Threading;
using System.Threading.Tasks;
using System.Xml;
using Kembec.SunatSdk;

namespace Kembec.SunatSdk.Vectors
{
    /// <summary>
    /// The battery of the package as a consumer restores it from its .nupkg: the 38 fixtures against their
    /// vectors (inclusive C14N, exact DigestValue and SignatureValue), the native version, zip and CDR, the
    /// errors, no_verdict without an answer and concurrent calls. Exit code 0 only if every check passed.
    /// </summary>
    internal static class Program
    {
        private const int ExpectedFixtures = 38;
        private static int _checks;
        private static int _failures;

        private static int Main(string[] args)
        {
            if (args.Length != 1)
            {
                Console.Error.WriteLine("usage: Vectors.CSharp <repository>");
                return 2;
            }
            var repo = Path.GetFullPath(args[0]);
            var key = new SigningKey(
                Path.Combine(repo, "tests", "vectors", "test-cert", "test-cert.pem"),
                Path.Combine(repo, "tests", "vectors", "test-cert", "test-key.pem"));
            var client = new SunatSdkClient(TimeSpan.FromSeconds(30));

            Run("version", () => Version());
            var fixtures = Directory.GetFiles(Path.Combine(repo, "fixtures"), "*.json").OrderBy(p => p, StringComparer.Ordinal).ToArray();
            Check(fixtures.Length == ExpectedFixtures, "fixtures: " + fixtures.Length + ", expected " + ExpectedFixtures);
            foreach (var fixture in fixtures)
            {
                Run(Path.GetFileNameWithoutExtension(fixture), () => Vector(client, key, repo, fixture));
            }
            Run("zip", () => Zip(client, key, repo));
            Run("cdr", () => ReadCdr(client, repo));
            Run("errors", () => Errors(client, key, repo));
            Run("no_verdict", () => NoVerdicts());
            Run("diagnosis", () => Diagnoses(repo));
            Run("threads", () => Threads(client, key, repo));

            var runtime = (IntPtr.Size * 8) + "-bit " + System.Runtime.InteropServices.RuntimeInformation.FrameworkDescription;
            if (_failures > 0)
            {
                Console.Error.WriteLine("C#: " + _failures + " of " + _checks + " checks failed (" + runtime + ")");
                return 1;
            }
            Console.WriteLine("ok C#: " + _checks + " checks, " + fixtures.Length + " vectors, " + runtime + ", native " + SunatSdkClient.NativeVersion);
            return 0;
        }

        private static void Version()
        {
            var informational = typeof(SunatSdkClient).Assembly.GetCustomAttribute<AssemblyInformationalVersionAttribute>();
            var package = (informational?.InformationalVersion ?? "").Split('+')[0];
            Check(SunatSdkClient.NativeVersion == package, "native " + SunatSdkClient.NativeVersion + ", package " + package);
        }

        private static void Vector(SunatSdkClient client, SigningKey key, string repo, string fixture)
        {
            var name = Path.GetFileNameWithoutExtension(fixture);
            var document = File.ReadAllText(fixture, Encoding.UTF8);
            var built = client.Build(document);
            Check(Canonical(Read(repo, name, "unsigned")) == Canonical(built.Xml), name + ": unsigned XML differs from its vector under C14N");
            Check(built.FileName == ExpectedFileName(document), name + ": file name " + built.FileName);

            var signed = client.Sign(built.Xml, key);
            var expected = Read(repo, name, "signed");
            Check(Canonical(expected) == Canonical(signed.Xml), name + ": signed XML differs from its vector under C14N");
            Check(expected.Contains("<ds:DigestValue>" + signed.DigestValue + "</ds:DigestValue>"), name + ": DigestValue " + signed.DigestValue);
            Check(expected.Contains("<ds:SignatureValue>" + signed.SignatureValue + "</ds:SignatureValue>"), name + ": SignatureValue differs");
            Check(signed.Xml.Contains("<ds:DigestValue>" + signed.DigestValue + "</ds:DigestValue>"), name + ": DigestValue not in the signed XML");
        }

        private static void Zip(SunatSdkClient client, SigningKey key, string repo)
        {
            var signed = SignFixture(client, key, repo, "01-invoice-taxed-cash");
            var zip = client.Zip("20123456789-01-F001-1", signed);
            Check(zip.FileName == "20123456789-01-F001-1.zip", "zip: file name " + zip.FileName);
            Check(zip.Content.Length > 4 && zip.Content[0] == (byte)'P' && zip.Content[1] == (byte)'K', "zip: not a zip");
        }

        private static void ReadCdr(SunatSdkClient client, string repo)
        {
            var bytes = File.ReadAllBytes(Path.Combine(repo, "crates", "sunat-transport", "tests", "data", "R-20150147718-01-FT01-982.zip"));
            var cdr = client.ReadCdr(bytes);
            Check(cdr.ReferenceId == "FT01-982", "cdr: reference " + cdr.ReferenceId);
            Check(cdr.ResponseCode == "2047", "cdr: code " + cdr.ResponseCode);
            Check(cdr.Class == ResponseClass.Rejected && !cdr.IsValidDocument, "cdr: class " + cdr.Class);
        }

        private static void Errors(SunatSdkClient client, SigningKey key, string repo)
        {
            Expect<SunatSdkException>(() => client.Build("{}"), e => e.Command == "build", "build of {} is the SDK's error");
            Expect<ArgumentException>(() => client.Build("{not json"), e => e.ParamName == "documentJson", "invalid JSON is an ArgumentException");
            Expect<ArgumentOutOfRangeException>(() => new SunatSdkClient(TimeSpan.FromMilliseconds(1500)), e => true, "timeout of 1.5 s");
            var sol = new SolCredentials("20123456789MODDATOS", "moddatos-secret");
            Check(!sol.ToString().Contains("moddatos-secret"), "SolCredentials.ToString shows the password");
            var signed = SignFixture(client, key, repo, "01-invoice-taxed-cash");
            Expect<SunatSdkException>(
                () => client.Send("20123456789-01-F001-1", signed, sol, "http://127.0.0.1/billService"),
                e => e.Command == "send" && e.Message.Contains("endpoint must use https://") && !e.Message.Contains("moddatos-secret"),
                "send over http://");
        }

        /// <summary>
        /// The core's answers (tests/data, the ones the PHP and wasm bindings check): the beta's real 2800 to a boleta,
        /// a CDR with two coded notes, and an answer without <c>diagnosis</c>. The result types have no public
        /// constructor, so they are built the way the client builds them, from the response JSON.
        /// </summary>
        private static void Diagnoses(string repo)
        {
            var sent = Parse<SendResult>(repo, "send-beta-2800.json");
            Check(sent.Outcome == Outcome.Fault && sent.Fault!.Code == "2800", "diagnosis: the 2800 is a fault");
            Check(sent.Diagnosis.Count == 1 && sent.Diagnosis[0].Code == "2800", "diagnosis: one element, 2800");
            var detail = sent.Diagnosis[0].Detail;
            Check(detail != null && detail.Node == "cbc:ID/schemeID" && detail.Value == "1", "diagnosis: detail cbc:ID/schemeID = 1");
            var fields = sent.Diagnosis[0].Fields;
            Check(fields.Count == 1 && fields[0].InputPath == "customer.document_type" && fields[0].Certainty == DiagnosisField.Source
                && fields[0].Xpath!.EndsWith("cbc:ID@schemeID", StringComparison.Ordinal), "diagnosis: customer.document_type, fuente");

            var status = Parse<StatusResult>(repo, "status-observed-4287.json");
            Check(status.Diagnosis.Select(d => d.Code).SequenceEqual(new[] { "4287", "4001" }), "diagnosis: 4287 and 4001");
            Check(status.Diagnosis[0].Detail == null && status.Diagnosis[0].Fields[0].InputPath == "lines[].unit_price"
                && status.Diagnosis[0].Fields[0].Certainty == DiagnosisField.Inferred, "diagnosis: 4287 points at lines[].unit_price, inferido");
            Check(status.Diagnosis[1].Fields.Count == 0, "diagnosis: 4001 is not in the rules");

            var without = Construct<SummaryResult>("{\"outcome\":\"ticket\",\"ticket\":\"1700000000000\"}");
            Check(without.Diagnosis.Count == 0, "diagnosis: absent is an empty list");
            var ticket = Construct<TicketResult>("{\"outcome\":\"fault\",\"code\":\"0109\",\"message\":\"x\",\"class\":null}");
            Check(ticket.Diagnosis.Count == 0, "diagnosis: a fault without it is empty");
        }

        private static T Parse<T>(string repo, string file)
            where T : SdkResult
        {
            return Construct<T>(File.ReadAllText(Path.Combine(repo, "bindings", "dotnet", "tests", "data", file), Encoding.UTF8));
        }

        private static T Construct<T>(string json)
            where T : SdkResult
        {
            using (var document = JsonDocument.Parse(json))
            {
                var constructor = typeof(T).GetConstructor(BindingFlags.Instance | BindingFlags.NonPublic, null, new[] { typeof(JsonElement), typeof(string) }, null)!;
                return (T)constructor.Invoke(new object[] { document.RootElement.Clone(), json });
            }
        }

        /// <summary>A listener that accepts and never answers the TLS handshake: every query waits out its 1 s.</summary>
        private static void NoVerdicts()
        {
            var listener = new TcpListener(IPAddress.Loopback, 0);
            listener.Start();
            var held = new List<TcpClient>();
            var accepting = new Thread(() =>
            {
                try
                {
                    while (true)
                    {
                        var socket = listener.AcceptTcpClient();
                        lock (held)
                        {
                            held.Add(socket);
                        }
                    }
                }
                catch (SocketException)
                {
                }
                catch (ObjectDisposedException)
                {
                }
            }) { IsBackground = true };
            accepting.Start();
            try
            {
                var port = ((IPEndPoint)listener.LocalEndpoint).Port;
                var silent = "https://127.0.0.1:" + port;
                var client = new SunatSdkClient(TimeSpan.FromSeconds(1));
                var sol = new SolCredentials("20123456789MODDATOS", "moddatos");
                var api = new ApiCredentials("client-id", "client-secret");

                var status = client.Status("20123456789", "01", "F001", 1, sol, silent + "/billConsultService");
                Check(status.Outcome == Outcome.NoVerdict && status.NoVerdict!.Reason == NoVerdictReason.Timeout && status.NoVerdict.HttpStatus == null,
                    "status without an answer: " + status.RawJson);

                var validity = client.Validity("20123456789", "01", "F001", 1, new DateTime(2026, 10, 1), 118.00m, api,
                    tokenEndpoint: silent + "/token", endpoint: silent + "/query");
                Check(validity.Outcome == Outcome.NoVerdict && validity.NoVerdict!.Stage == ExchangeStage.Token,
                    "validity without a token: " + validity.RawJson);

                var gre = client.GreTicket("1700000000000", api, sol, silent + "/token", silent + "/v1");
                Check(gre.Outcome == Outcome.NoVerdict && gre.NoVerdict!.Stage == ExchangeStage.Token && gre.Ticket == "1700000000000",
                    "gre-ticket without a token: " + gre.RawJson);
            }
            finally
            {
                listener.Stop();
                lock (held)
                {
                    held.ForEach(socket => socket.Close());
                }
            }
        }

        /// <summary>Eight threads build and sign different fixtures at once; each must still match its vector.</summary>
        private static void Threads(SunatSdkClient client, SigningKey key, string repo)
        {
            var names = Directory.GetFiles(Path.Combine(repo, "fixtures"), "*.json")
                .Select(path => Path.GetFileNameWithoutExtension(path)!).OrderBy(n => n, StringComparer.Ordinal).Take(8).ToArray();
            var results = new string?[names.Length];
            var tasks = names.Select((name, i) => Task.Factory.StartNew(() =>
            {
                for (var round = 0; round < 5; round++)
                {
                    results[i] = client.Sign(client.Build(File.ReadAllText(Path.Combine(repo, "fixtures", name + ".json"), Encoding.UTF8)).Xml, key).DigestValue;
                }
            }, TaskCreationOptions.LongRunning)).ToArray();
            Task.WaitAll(tasks);
            for (var i = 0; i < names.Length; i++)
            {
                Check(Read(repo, names[i], "signed").Contains("<ds:DigestValue>" + results[i] + "</ds:DigestValue>"), "threads: " + names[i]);
            }
        }

        private static string SignFixture(SunatSdkClient client, SigningKey key, string repo, string name)
        {
            var built = client.Build(File.ReadAllText(Path.Combine(repo, "fixtures", name + ".json"), Encoding.UTF8));
            return client.Sign(built.Xml, key).Xml;
        }

        /// <summary><c>ruc-type-series-number</c>, or <c>ruc-RC|RA-YYYYMMDD-number</c>, from the fixture's own fields.</summary>
        private static string ExpectedFileName(string documentJson)
        {
            using (var document = JsonDocument.Parse(documentJson))
            {
                var root = document.RootElement;
                var ruc = root.GetProperty("issuer").GetProperty("ruc").GetString();
                var number = root.GetProperty("number").GetString();
                switch (root.GetProperty("kind").GetString())
                {
                    case "summary":
                        return ruc + "-RC-" + root.GetProperty("generation_date").GetString()!.Replace("-", "") + "-" + number;
                    case "voided":
                        return ruc + "-RA-" + root.GetProperty("communication_date").GetString()!.Replace("-", "") + "-" + number;
                    default:
                        return ruc + "-" + root.GetProperty("document_type").GetString() + "-" + root.GetProperty("series").GetString() + "-" + number;
                }
            }
        }

        /// <summary>Inclusive C14N 1.0 without comments, the form the signature is computed over.</summary>
        private static string Canonical(string xml)
        {
            var document = new XmlDocument { PreserveWhitespace = true, XmlResolver = null };
            document.LoadXml(xml);
            var transform = new XmlDsigC14NTransform(false);
            transform.LoadInput(document);
            using (var stream = (Stream)transform.GetOutput(typeof(Stream)))
            using (var reader = new StreamReader(stream, new UTF8Encoding(false)))
            {
                return reader.ReadToEnd();
            }
        }

        private static string Read(string repo, string name, string kind)
        {
            return File.ReadAllText(Path.Combine(repo, "tests", "vectors", name + "." + kind + ".xml"), Encoding.UTF8);
        }

        private static void Expect<T>(Action action, Func<T, bool> holds, string what)
            where T : Exception
        {
            try
            {
                action();
                Check(false, what + ": no exception");
            }
            catch (T e) when (e.GetType() == typeof(T))
            {
                Check(holds(e), what + ": " + e.Message);
            }
            catch (Exception e)
            {
                Check(false, what + ": " + e.GetType().Name + " " + e.Message);
            }
        }

        private static void Run(string what, Action test)
        {
            try
            {
                test();
            }
            catch (Exception e)
            {
                Check(false, what + ": " + e.GetType().Name + " " + e.Message);
            }
        }

        private static void Check(bool ok, string what)
        {
            Interlocked.Increment(ref _checks);
            if (!ok)
            {
                Interlocked.Increment(ref _failures);
                Console.Error.WriteLine("FAIL " + what);
            }
        }
    }
}
