using System;
using System.Buffers;

namespace Kembec.SunatSdk.Interop
{
    /// <summary>
    /// The request's bytes, SOL password and API secret included. Every array it outgrows and the last
    /// one are zeroed (docs/c-abi.md: the caller clears the request after the call), so no copy of a
    /// secret stays behind in a buffer the garbage collector frees later.
    /// </summary>
    internal sealed class ClearingBuffer : IBufferWriter<byte>, IDisposable
    {
        private byte[] _buffer;
        private int _written;

        internal ClearingBuffer(int capacity)
        {
            _buffer = new byte[Math.Max(capacity, 256)];
        }

        /// <summary>The bytes written; valid until <see cref="Dispose"/>.</summary>
        internal byte[] Array => _buffer;

        internal int Length => _written;

        public void Advance(int count)
        {
            if (count < 0 || _written + count > _buffer.Length)
            {
                throw new ArgumentOutOfRangeException(nameof(count));
            }
            _written += count;
        }

        public Memory<byte> GetMemory(int sizeHint = 0)
        {
            Reserve(sizeHint);
            return _buffer.AsMemory(_written);
        }

        public Span<byte> GetSpan(int sizeHint = 0)
        {
            Reserve(sizeHint);
            return _buffer.AsSpan(_written);
        }

        public void Dispose()
        {
            System.Array.Clear(_buffer, 0, _buffer.Length);
            _written = 0;
        }

        private void Reserve(int sizeHint)
        {
            var needed = _written + Math.Max(sizeHint, 1);
            if (needed <= _buffer.Length)
            {
                return;
            }
            var grown = new byte[Math.Max(needed, _buffer.Length * 2)];
            System.Buffer.BlockCopy(_buffer, 0, grown, 0, _written);
            System.Array.Clear(_buffer, 0, _buffer.Length);
            _buffer = grown;
        }
    }
}
