use crate::model::{Id,Project,Target};
use std::collections::{HashMap,HashSet};

#[derive(Clone,Debug)]
pub struct Placement {pub id:Id,pub position:[f32;2]}

/// A complete forest layout; supplementary links never duplicate nodes.
pub fn layout(project:&Project)->Vec<Placement> {
    let mut children:HashMap<Id,Vec<Id>>=HashMap::new();
    for c in &project.connections {if !c.supplementary {if let Target::Port(to)=c.to{children.entry(c.from.node).or_default().push(to.node);}}}
    let mut seen=HashSet::new();let mut result=Vec::new();let mut widths=HashMap::new();let mut cursor=0.;
    let mut roots=project.roots.clone();roots.extend(project.nodes.iter().map(|n|n.id));
    for root in roots {
        if seen.contains(&root){continue;}
        let mut order=vec![];let mut stack=vec![(root,0)];
        while let Some((id,depth))=stack.pop(){if !seen.insert(id){continue;}order.push((id,depth));if let Some(kids)=children.get(&id){for child in kids.iter().rev(){stack.push((*child,depth+1));}}}
        for (id,_) in order.iter().rev(){let width=children.get(id).map(|kids|kids.iter().map(|c|widths.get(c).copied().unwrap_or(1.)).sum::<f32>()).unwrap_or(1.).max(1.);widths.insert(*id,width);}
        let mut positions=HashMap::from([(root,cursor)]);
        for (id,depth) in order {
            let start=positions.get(&id).copied().unwrap_or(cursor);let width=widths[&id];
            result.push(Placement{id,position:[(start+width/2.)*340.,depth as f32*310.]});
            let mut child_start=start;if let Some(kids)=children.get(&id){for child in kids{positions.insert(*child,child_start);child_start+=widths.get(child).copied().unwrap_or(1.);}}
        }
        cursor+=widths.get(&root).copied().unwrap_or(1.)+0.35;
    }result
}
