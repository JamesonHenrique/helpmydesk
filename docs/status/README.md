# Andamento

`roadmap.json` lista os itens do roteiro (plano v1, Parte X) com fase, estado (`todo`, `doing`,
`done`), peso (1 a 3, pelo tamanho), data de entrega e commit. É a fonte do painel de andamento.

`waiting_on` diz, num item aberto, o que só o mantenedor destrava (teste em máquina real, uma
chave, uma decisão); vazio quando o próximo passo não depende dele. O painel mostra esses itens à
parte, para o percentual parado não parecer trabalho parado.

Percentual entregue = soma dos pesos dos itens `done` ÷ soma de todos os pesos. Um item em
andamento só conta quando é entregue.

A cada entrega: atualize o item aqui no mesmo commit da entrega.
