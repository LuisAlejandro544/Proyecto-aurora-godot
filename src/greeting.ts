/**
 * Lógica pura del saludo para el entorno web.
 * Se mantiene separada del componente visual para permitir pruebas unitarias rápidas.
 */
export function getGreeting(): string {
  return 'Hello World';
}
