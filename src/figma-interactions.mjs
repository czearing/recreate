import fs from 'node:fs';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { figmaGuid as guid } from './figma-node.mjs';

export function writeFigmaInteractions({
  outDir,
  nodes,
  pageForNode,
  reference,
}) {
  const eventCounts = {};
  const navigationCounts = {};
  const transitionCounts = {};
  const flows = [];
  for (const node of nodes) {
    const interactions = (node.prototypeInteractions || [])
      .filter((interaction) =>
        !interaction.isDeleted &&
        interaction.event &&
        Object.keys(interaction.event).length > 0
      );
    if (!interactions.length) continue;
    const summaries = interactions.map((interaction) => {
      const eventType = interaction.event?.interactionType || 'UNKNOWN';
      eventCounts[eventType] = (eventCounts[eventType] || 0) + 1;
      const actions = (interaction.actions || []).map((action) => {
        const navigation = action.navigationType || 'NONE';
        const transition = action.transitionType || 'NONE';
        navigationCounts[navigation] = (navigationCounts[navigation] || 0) + 1;
        transitionCounts[transition] = (transitionCounts[transition] || 0) + 1;
        return {
          destinationId: guid(action.transitionNodeID),
          navigationType: action.navigationType,
          connectionType: action.connectionType,
          transitionType: action.transitionType,
          duration: action.transitionDuration,
          easingType: action.easingType,
          easingFunction: action.easingFunction,
          preserveScroll: action.transitionPreserveScroll,
          resetScroll: action.transitionResetScrollPosition,
          overlayPositionType: action.overlayPositionType,
          overlayRelativePosition: action.overlayRelativePosition,
        };
      });
      return {
        id: guid(interaction.id),
        eventType,
        event: reference(interaction.event),
        actions,
      };
    });
    flows.push({
      nodeId: guid(node.guid),
      pageId: pageForNode.get(guid(node.guid)),
      type: node.type,
      name: node.name,
      interactions: summaries,
      exact: reference(interactions),
    });
  }
  const shardDirectory = path.join(outDir, 'evidence', 'figma', 'interactions');
  fs.mkdirSync(shardDirectory, { recursive: true });
  const shards = {};
  const facets = {
    values: 'flow indexes',
    navigationTypes: {},
    transitionTypes: {},
    timeoutSeconds: {},
  };
  const search = flows.map((flow, flowIndex) => {
    const actions = flow.interactions.flatMap(
      (interaction) => interaction.actions,
    );
    for (const type of new Set(
      actions.map((action) => action.navigationType || 'NONE'),
    )) (facets.navigationTypes[type] ||= []).push(flowIndex);
    for (const type of new Set(
      actions.map((action) => action.transitionType || 'NONE'),
    )) (facets.transitionTypes[type] ||= []).push(flowIndex);
    const timeoutSeconds = [...new Set(
      flow.interactions
        .map((interaction) => interaction.event?.transitionTimeout)
        .filter((value) => Number.isFinite(value)),
    )];
    if (timeoutSeconds.length) facets.timeoutSeconds[flowIndex] = timeoutSeconds;
    const prefix = createHash('sha256')
      .update(flow.nodeId)
      .digest('hex')[0];
    shards[prefix] ||= {};
    shards[prefix][flow.nodeId] = flow;
    return {
      nodeId: flow.nodeId,
      pageId: flow.pageId,
      type: flow.type,
      name: flow.name,
      eventTypes: [...new Set(
        flow.interactions.map((interaction) => interaction.eventType),
      )],
      interactionCount: flow.interactions.length,
      detailPrefix: prefix,
    };
  });
  const searchFile = 'evidence/figma/interaction-search.json';
  fs.writeFileSync(
    path.join(outDir, searchFile),
    JSON.stringify({ flows: search, facets }),
  );
  const shardFiles = [];
  for (const [prefix, shardFlows] of Object.entries(shards)) {
    const file = `evidence/figma/interactions/${prefix}.json`;
    fs.writeFileSync(
      path.join(outDir, file),
      JSON.stringify({ flows: shardFlows }),
    );
    shardFiles.push(file);
  }
  return {
    nodeCount: flows.length,
    interactionCount: flows.reduce(
      (count, flow) => count + flow.interactions.length,
      0,
    ),
    eventCounts,
    navigationCounts,
    transitionCounts,
    search: searchFile,
    details: {
      pattern: 'evidence/figma/interactions/<detail-prefix>.json',
      shards: shardFiles.sort(),
    },
  };
}
