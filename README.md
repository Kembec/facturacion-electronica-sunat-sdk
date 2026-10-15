# facturacion-electronica-sunat-sdk

SDK para emitir comprobantes de pago electrónicos ante la SUNAT (Perú) desde un único núcleo en Rust,
con builds para PHP, JavaScript/WebAssembly, Python y Go.

**Estado: diseño.** Todavía no hay código utilizable.

## Qué resuelve

- Un solo núcleo arma el XML UBL 2.1, lo firma, lo empaqueta, lo envía y lee la constancia de
  recepción (CDR). Cada lenguaje obtiene exactamente el mismo resultado.
- Cada lenguaje recibe un build nativo o WebAssembly, también para entornos edge como Cloudflare
  Workers, sin depender de un servicio aparte en otro lenguaje.

## Compatibilidad

Un tipo de documento se incorpora solo si, para cada caso de `fixtures/`, su XML coincide con los
vectores de compatibilidad de `tests/vectors/` (forma canónica, `DigestValue` y `SignatureValue`) y el
ambiente beta de la SUNAT lo acepta.

## Cuándo un sistema puede usarlo

Un sistema en producción adopta una versión solo cuando esa versión pasa toda la batería del documento
que va a emitir: equivalencia con los vectores, validación de estructura, verificación de la firma y
aceptación en el ambiente beta de la SUNAT. Cada adopción se hace con el build del lenguaje de ese
sistema y con una vía de vuelta a la implementación anterior.

## Orden de incorporación

1. Factura, boleta, notas de crédito y débito; firma XML; lectura de la CDR.
2. Resumen diario y comunicación de baja.
3. Guía de remisión electrónica (API REST).
4. Retención y percepción.

## Derechos y licencia

- Código de este repositorio: licencia MIT, © 2026 Manuel Benancio (ver `LICENSE`).
- Documentación técnica, catálogos y esquemas de la SUNAT: pertenecen a la SUNAT. Este repositorio
  enlaza a sus fuentes oficiales y no los redistribuye.
- Proyecto independiente: no está afiliado a la SUNAT, y no es un PSE ni un OSE. Quien
  lo usa sigue siendo el emisor electrónico y responde por sus comprobantes.
- Se entrega sin garantía, según la licencia MIT.
