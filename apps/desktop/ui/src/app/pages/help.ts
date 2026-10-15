// SPDX-License-Identifier: GPL-3.0-or-later
// Help center: what the app does, its declared limits (Plan v1 III.10) and how to contribute.
import { Component } from '@angular/core';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'app-help',
  imports: [RouterLink],
  template: `
    <section class="page" aria-labelledby="t">
      <h1 id="t">Ajuda</h1>
      <div class="card">
        <h2>O que o HelpMyDesk faz</h2>
        <p>
          Verifica problemas comuns de internet, lentidão, espaço, som, microfone, câmera e
          programas; explica o que encontrou; e só faz correções conhecidas, com a sua aprovação,
          conferindo depois se resolveu.
        </p>
      </div>
      <div class="card">
        <h2>O que ele não faz</h2>
        <ul class="steps">
          <li>
            Não conserta hardware, vírus ou disco com defeito: nesses casos prepara um relatório.
          </li>
          <li>Não instala nem remove programas e não mexe no antivírus ou no firewall.</li>
          <li>Não acessa seu computador pela internet nem envia dados para o projeto.</li>
          <li>
            Pode errar: por isso diz quando não tem certeza e nunca faz correções arriscadas sem
            perguntar.
          </li>
        </ul>
      </div>
      <div class="card">
        <h2>Ajuda inteligente (opcional)</h2>
        <p>
          Com ela ligada, você escreve o problema com as suas palavras e a IA só escolhe o tipo de
          problema; as verificações e as correções continuam as mesmas. Usa a sua conta gratuita do
          OpenCode, e o texto vai sem seu nome nem o do computador. Ligue ou desligue em
          <a routerLink="/configuracoes">Configurações</a>.
        </p>
      </div>
      <div class="card">
        <h2>Quando não resolver</h2>
        <p>
          Use <a routerLink="/relatorio">"Não sei explicar"</a> para gerar um relatório e levar a
          quem for ajudar.
        </p>
      </div>
      <div class="card">
        <h2>Contribuir</h2>
        <p>
          O HelpMyDesk é software livre (GPL-3.0). Código e instruções:
          github.com/JamesonHenrique/helpmydesk.
        </p>
      </div>
    </section>
  `,
})
export class HelpPage {}
