// SPDX-License-Identifier: GPL-3.0-or-later
// Help center: what the app does, its declared limits (Plan v1 III.10) and how to contribute.
import { Component } from '@angular/core';
import { RouterLink } from '@angular/router';

@Component({
  selector: 'app-help',
  imports: [RouterLink],
  template: `
    <section class="page" aria-labelledby="t">
      <div class="page-head">
        <p class="rule">Guia de uso · 01 a 05</p>
        <h1 id="t">Ajuda</h1>
        <p class="lead">
          O essencial em cinco leituras curtas. Use o índice para pular direto ao ponto.
        </p>
      </div>
      <nav class="help-toc" aria-label="Índice do guia">
        <a href="#faz">01 · O que o HelpMyDesk faz</a>
        <a href="#nao-faz">02 · O que ele não faz</a>
        <a href="#ia">03 · Ajuda inteligente (opcional)</a>
        <a href="#nao-resolve">04 · Quando não resolver</a>
        <a href="#contrib">05 · Contribuir</a>
      </nav>
      <div class="guide">
        <article class="sheet" id="faz">
          <p class="rule help-rule">01</p>
          <h2 class="guide-title">O que o HelpMyDesk faz</h2>
          <p>
            Verifica problemas comuns de internet, lentidão, espaço, som, microfone, câmera e
            programas; explica o que encontrou; e só faz correções conhecidas, com a sua aprovação,
            conferindo depois se resolveu.
          </p>
        </article>
        <article class="sheet" id="nao-faz">
          <p class="rule help-rule">02</p>
          <h2 class="guide-title">O que ele não faz</h2>
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
        </article>
        <article class="sheet" id="ia">
          <p class="rule help-rule">03</p>
          <h2 class="guide-title">Ajuda inteligente (opcional)</h2>
          <p>
            Com ela ligada, você escreve o problema com as suas palavras e a IA só escolhe o tipo de
            problema; as verificações e as correções continuam as mesmas. Usa a sua conta gratuita
            do OpenCode, e o texto vai sem seu nome nem o do computador. Ligue ou desligue em
            <a routerLink="/configuracoes">Configurações</a>.
          </p>
        </article>
        <article class="sheet" id="nao-resolve">
          <p class="rule help-rule">04</p>
          <h2 class="guide-title">Quando não resolver</h2>
          <p>
            Use <a routerLink="/relatorio">"Não sei explicar"</a> para gerar um relatório e levar a
            quem for ajudar.
          </p>
        </article>
        <article class="sheet" id="contrib">
          <p class="rule help-rule">05</p>
          <h2 class="guide-title">Contribuir</h2>
          <p>
            O HelpMyDesk é software livre (GPL-3.0). Código e instruções:
            github.com/JamesonHenrique/helpmydesk.
          </p>
        </article>
      </div>
    </section>
  `,
  styles: `
    .help-toc {
      display: flex;
      flex-wrap: wrap;
      gap: 0.3rem 1.1rem;
      border-block: 1px solid var(--line);
      padding: 0.6rem 0;
    }
    .help-toc a {
      font-family: var(--font-mono);
      font-size: 0.85rem;
      color: var(--muted);
      text-decoration: none;
    }
    .help-toc a:hover {
      color: var(--accent);
      text-decoration: underline;
    }
    .guide {
      display: grid;
      gap: 1rem;
      max-width: 52rem;
    }
    .help-rule {
      margin-bottom: 0;
    }
    .guide-title {
      margin-top: 0.15rem;
    }
    article[id] {
      scroll-margin-top: 1rem;
    }
  `,
})
export class HelpPage {}
