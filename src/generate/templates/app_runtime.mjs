import React,{useEffect,useLayoutEffect,useRef,useState,useSyncExternalStore} from 'react';
import {createPortal} from 'react-dom';
import {reduceInteraction} from './runtime/interaction.mjs';
import {startSequences} from './runtime/sequence.mjs';
//__RECREATE_COMPONENT_IMPORT__
//__RECREATE_STATE_IMPORT__
const keyActivate=(event,action)=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();action(event)}};
const pathOf=element=>{const parts=[];for(let node=element;node&&node!==document.documentElement;node=node.parentElement){const peers=node.parentElement?[...node.parentElement.children].filter(child=>child.tagName===node.tagName):[node];parts.push(`${node.tagName.toLowerCase()}:nth-of-type(${peers.indexOf(node)+1})`)}return `html>${parts.reverse().join('>')}`};
const captureScroll=element=>{const elements=[];for(let node=element?.parentElement;node&&node!==document.documentElement;node=node.parentElement){if(node.scrollLeft||node.scrollTop)elements.push([pathOf(node),node.scrollLeft,node.scrollTop])}return{window:[scrollX,scrollY],elements}};
        const scrollAnimations=new WeakMap();const scrollEase=value=>{let current=value;for(let index=0;index<5;index++){const inverse=1-current;const x=3*inverse*inverse*current*.4+3*inverse*current*current*.2+current*current*current;const slope=3*inverse*inverse*.4+6*inverse*current*(.2-.4)+3*current*current*(1-.2);if(Math.abs(slope)<1e-4)break;current=Math.max(0,Math.min(1,current-(x-value)/slope))}const inverse=1-current;return 3*inverse*current*current+current*current*current};const setScroll=(element,left,top)=>element===window?scrollTo(left,top):element.scrollTo(left,top);const animateScroll=(element,left,top)=>{if(element!==window&&top===0){const content=[...element.children].find(child=>child.scrollWidth>element.clientWidth&&getComputedStyle(child).transition.includes('transform'));        if(content){element.scrollTo(0,0);requestAnimationFrame(()=>requestAnimationFrame(()=>{content.style.transform=`translateX(${-left}px)`}));return}}const startLeft=element===window?scrollX:element.scrollLeft;const startTop=element===window?scrollY:element.scrollTop;if(Math.abs(startLeft-left)<1&&Math.abs(startTop-top)<1)return;const token={};scrollAnimations.set(element,token);const started=performance.now();const frame=now=>{if(scrollAnimations.get(element)!==token)return;const progress=Math.min(1,(now-started)/320);const eased=scrollEase(progress);setScroll(element,startLeft+(left-startLeft)*eased,startTop+(top-startTop)*eased);if(progress<1)requestAnimationFrame(frame)};requestAnimationFrame(frame)};const restoreScroll=snapshot=>{if(snapshot.smooth){animateScroll(window,snapshot.window[0],snapshot.window[1]);snapshot.elements.forEach(([path,left,top])=>{const element=document.querySelector(path);if(element)animateScroll(element,left,top)});return}setScroll(window,snapshot.window[0],snapshot.window[1]);snapshot.elements.forEach(([path,left,top])=>{const element=document.querySelector(path);if(element)setScroll(element,left,top)})};
"__RECREATE_POSITIONAL__"
const viewportWidths=["__RECREATE_WIDTHS__"];
const closableStates=["__RECREATE_CLOSABLE__"];
const statefulStates=["__RECREATE_STATEFUL__"];
const replacementStates=["__RECREATE_REPLACEMENT_STATES__"];
const capturedScrolls="__RECREATE_SCROLL_TARGETS__";
const carouselState="__RECREATE_CAROUSEL_STATE__";
        const attributeSequences="__RECREATE_ATTRIBUTE_SEQUENCES__";
const responsiveAttributePaths="__RECREATE_RESPONSIVE_ATTRIBUTE_PATHS__";
const responsiveAttributeValues="__RECREATE_RESPONSIVE_ATTRIBUTE_VALUES__";
const responsiveAttributes="__RECREATE_RESPONSIVE_ATTRIBUTES__";
const capturedScroll=(state,viewport)=>capturedScrolls[state]?.[viewport]??null;
        const subscribe=notify=>{const media=viewportWidths.slice(1).map(width=>matchMedia(`(max-width:${width}px)`));media.forEach(query=>query.addEventListener('change',notify));addEventListener('resize',notify);return()=>{media.forEach(query=>query.removeEventListener('change',notify));removeEventListener('resize',notify)}};
//__RECREATE_VIEWS__
const baselineViews=["__RECREATE_VIEW_NAMES__"];
