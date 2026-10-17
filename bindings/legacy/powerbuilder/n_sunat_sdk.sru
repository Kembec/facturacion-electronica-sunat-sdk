$PBExportHeader$n_sunat_sdk.sru
$PBExportComments$SUNAT electronic invoicing SDK through the C ABI of sunat_sdk_ffi.dll (docs/c-abi.md)
forward
global type n_sunat_sdk from nonvisualobject
end type
end forward

global type n_sunat_sdk from nonvisualobject
end type
global n_sunat_sdk n_sunat_sdk

type prototypes
// stdcall with undecorated names. longptr is 4 bytes in a 32-bit process and 8 in a 64-bit one,
// like the DLL's pointers and size_t, so these lines serve both builds of the DLL.
// bindings/legacy/tests/declarations.py reads them and the CI calls the 32 and 64-bit DLLs through
// exactly these types; keep each one on one line.
function long sunat_sdk_call (ref blob request, ref longptr response) library "sunat_sdk_ffi.dll"
function longptr sunat_sdk_response_len (longptr response) library "sunat_sdk_ffi.dll"
function long sunat_sdk_response_copy (longptr response, ref blob buffer, longptr capacity) library "sunat_sdk_ffi.dll"
subroutine sunat_sdk_free (longptr response) library "sunat_sdk_ffi.dll"
function string sunat_sdk_version () library "sunat_sdk_ffi.dll" alias for "sunat_sdk_version;Ansi"
function ulong sunat_sdk_abi_version () library "sunat_sdk_ffi.dll"
function longptr LoadLibraryW (string path) library "kernel32.dll"
end prototypes

forward prototypes
public subroutine of_load (string as_dll)
public function string of_version ()
public function string of_call (string as_request)
private function string of_take_response (longptr al_response)
private function string of_error_message (string as_response)
private subroutine of_throw (string as_message)
end prototypes

public subroutine of_load (string as_dll);// Loads the DLL from a full path and checks its ABI; the external functions then bind to this
// module by name, so the DLL does not have to be next to the IDE or on the PATH.
// A process loads only the DLL of its own bitness: dll\x86 for a 32-bit IDE or runtime, dll\x64
// for a 64-bit one.
if LoadLibraryW(as_dll) = 0 then
	of_throw("cannot load " + as_dll + "; a 32-bit PowerBuilder needs dll\x86, a 64-bit one dll\x64")
end if
if sunat_sdk_abi_version() <> 1 then
	of_throw("sunat_sdk_ffi.dll is of another ABI version")
end if
end subroutine

public function string of_version ();// The SDK version of the loaded DLL, for example "0.1.0-beta.6".
return sunat_sdk_version()
end function

public function string of_call (string as_request);// Runs one command (docs/c-abi.md) and returns its JSON response. A rejected command throws a
// RuntimeError with the SDK's message. A SOAP fault or a query without verdict is not an error:
// it is the "outcome" of the response, as in the CLI.
//
// Text crosses the DLL as UTF-8 bytes in a blob: a string argument would go as UTF-16, or as the
// ANSI code page with ;Ansi, and lose every character outside it.
blob lb_utf8, lb_request
longptr ll_response = 0
long ll_status, ll_size
string ls_response

lb_utf8 = Blob(as_request, EncodingUTF8!)
ll_size = Len(lb_utf8)
// The request ends with a NUL: a blob one byte longer, the request copied in, the last byte 0.
lb_request = Blob(Space(ll_size + 1), EncodingANSI!)
BlobEdit(lb_request, 1, lb_utf8)
SetByte(lb_request, ll_size + 1, 0)

ll_status = sunat_sdk_call(lb_request, ll_response)
if ll_response = 0 then
	of_throw("sunat_sdk_call gave no response")
end if
ls_response = of_take_response(ll_response)
if ll_status <> 0 then
	of_throw(of_error_message(ls_response))
end if
return ls_response
end function

private function string of_take_response (longptr al_response);// Copies the response out of the library (PowerBuilder cannot read through a pointer) and
// releases it right after, with the library's own sunat_sdk_free.
longptr ll_length
long ll_copied
blob lb_buffer

ll_length = sunat_sdk_response_len(al_response)
lb_buffer = Blob(Space(ll_length + 1), EncodingANSI!)
ll_copied = sunat_sdk_response_copy(al_response, lb_buffer, ll_length + 1)
sunat_sdk_free(al_response)
if ll_copied <> 0 then
	of_throw("sunat_sdk_response_copy failed")
end if
return String(BlobMid(lb_buffer, 1, ll_length), EncodingUTF8!)
end function

private function string of_error_message (string as_response);// The "error" of a rejected command; it never carries a password or a secret.
JSONParser lnv_json
string ls_message

lnv_json = create JSONParser
if lnv_json.LoadString(as_response) = "" then
	ls_message = lnv_json.GetItemString(lnv_json.GetRootItem(), "error")
else
	ls_message = as_response
end if
destroy lnv_json
return ls_message
end function

private subroutine of_throw (string as_message);RuntimeError lre_error

lre_error = create RuntimeError
lre_error.SetMessage("sunat_sdk: " + as_message)
throw lre_error
end subroutine

on n_sunat_sdk.create
call super::create
TriggerEvent( this, "constructor" )
end on

on n_sunat_sdk.destroy
TriggerEvent( this, "destructor" )
call super::destroy
end on
