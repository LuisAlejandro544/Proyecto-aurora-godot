/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import { getGreeting } from './greeting';

/**
 * Componente raíz para el entorno web.
 * Renderiza únicamente un "Hello World" limpio para mantener la compilación web operativa
 * mientras el proyecto móvil se estructura con Godot 4.7 y Rust.
 */
export default function App() {
  return (
    <main className="flex min-h-screen items-center justify-center bg-neutral-950 text-neutral-100">
      <h1 className="text-3xl font-semibold tracking-tight">{getGreeting()}</h1>
    </main>
  );
}
