use std::io::{Read, Result as IoResult, Seek, SeekFrom, Write};
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::fs::File as TokioFile;

/// Дескриптор виртуального файла для чтения
/// Содержит синхронный дескриптор (для обычного чтения) и асинхронный (для Tokio) [1].
pub struct VFileDesc {
    pub sync_handle: std::fs::File,
    pub async_handle: TokioFile,
}

/// Дескриптор изменяемого файла (только для /data/)
pub struct VFileDescMut {
    pub sync_handle: std::fs::File,
    pub async_handle: TokioFile,
}

// --- Синхронная реализация (Стандартная библиотека) ---

impl Read for VFileDesc {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.sync_handle.read(buf)
    }
}

impl Seek for VFileDesc {
    fn seek(&mut self, pos: SeekFrom) -> IoResult<u64> {
        self.sync_handle.seek(pos)
    }
}

impl Read for VFileDescMut {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        self.sync_handle.read(buf)
    }
}

impl Write for VFileDescMut {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        self.sync_handle.write(buf)
    }
    fn flush(&mut self) -> IoResult<()> {
        self.sync_handle.flush()
    }
}

// --- Асинхронная реализация (Трейты Tokio) ---

// Реализуем чтение для дескриптора чтения [1]
impl tokio::io::AsyncRead for VFileDesc {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        Pin::new(&mut self.async_handle).poll_read(cx, buf)
    }
}

// Реализуем чтение для изменяемого дескриптора [1]
impl tokio::io::AsyncRead for VFileDescMut {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<IoResult<()>> {
        Pin::new(&mut self.async_handle).poll_read(cx, buf)
    }
}

// Реализуем запись для изменяемого дескриптора [1]
impl tokio::io::AsyncWrite for VFileDescMut {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<IoResult<usize>> {
        Pin::new(&mut self.async_handle).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.async_handle).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<IoResult<()>> {
        Pin::new(&mut self.async_handle).poll_shutdown(cx)
    }
}
