use super::id::PaneId;

#[derive(Debug, Clone, PartialEq)]
pub enum LayoutSplit {
    Horizontal, // {}
    Vertical,   // []
}

#[derive(Debug, Clone, PartialEq)]
pub enum LayoutNode {
    Leaf {
        width: u16,
        height: u16,
        x: u16,
        y: u16,
        pane_id: Option<PaneId>,
    },
    Container {
        split: LayoutSplit,
        width: u16,
        height: u16,
        x: u16,
        y: u16,
        children: Vec<LayoutNode>,
    },
}

impl LayoutNode {
    /// Parse a tmux window_layout string into a LayoutNode tree
    /// Example: "bb62,204x50,0,0{101x50,0,0,1,102x50,102,0[102x24,102,0,2,102x25,102,25,3]}"
    pub fn parse(layout_str: &str) -> Option<Self> {
        let layout = layout_str.trim();
        if layout.is_empty() {
            return None;
        }

        // Layout usually starts with a checksum, e.g. "bb62,"
        let body = if let Some((_, rest)) = layout.split_once(',') {
            rest
        } else {
            layout
        };

        let mut chars = body.chars().peekable();
        parse_node(&mut chars)
    }

    pub fn width(&self) -> u16 {
        match self {
            LayoutNode::Leaf { width, .. } => *width,
            LayoutNode::Container { width, .. } => *width,
        }
    }

    pub fn height(&self) -> u16 {
        match self {
            LayoutNode::Leaf { height, .. } => *height,
            LayoutNode::Container { height, .. } => *height,
        }
    }

    pub fn dimension(&self, split: &LayoutSplit) -> u16 {
        match split {
            LayoutSplit::Horizontal => self.width(),
            LayoutSplit::Vertical => self.height(),
        }
    }

    pub fn find_pane_at(&self, area: ratatui::layout::Rect, x: u16, y: u16) -> Option<PaneId> {
        if x < area.x || x >= area.x + area.width || y < area.y || y >= area.y + area.height {
            return None;
        }

        match self {
            LayoutNode::Leaf { pane_id, .. } => pane_id.clone(),
            LayoutNode::Container { split, children, .. } => {
                if children.is_empty() {
                    return None;
                }

                let dir = match split {
                    LayoutSplit::Horizontal => ratatui::layout::Direction::Horizontal,
                    LayoutSplit::Vertical => ratatui::layout::Direction::Vertical,
                };

                let total_dim: u32 = children.iter().map(|c| c.dimension(split) as u32).sum();
                let constraints: Vec<ratatui::layout::Constraint> = children
                    .iter()
                    .map(|c| {
                        let dim = c.dimension(split) as u32;
                        if total_dim > 0 {
                            ratatui::layout::Constraint::Ratio(dim.max(1), total_dim.max(1))
                        } else {
                            ratatui::layout::Constraint::Ratio(1, children.len() as u32)
                        }
                    })
                    .collect();

                let chunks = ratatui::layout::Layout::default()
                    .direction(dir)
                    .constraints(constraints)
                    .split(area);

                for (idx, child) in children.iter().enumerate() {
                    if idx < chunks.len()
                        && let Some(p_id) = child.find_pane_at(chunks[idx], x, y)
                    {
                        return Some(p_id);
                    }
                }

                None
            }
        }
    }
}

fn parse_number<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> Option<u16> {
    let mut num_str = String::new();
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            num_str.push(c);
            chars.next();
        } else {
            break;
        }
    }
    if num_str.is_empty() {
        None
    } else {
        num_str.parse::<u16>().ok()
    }
}

fn parse_node<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> Option<LayoutNode> {
    // 1. Parse width
    let width = parse_number(chars)?;

    // 2. Expect 'x'
    if chars.next()? != 'x' {
        return None;
    }

    // 3. Parse height
    let height = parse_number(chars)?;

    // 4. Expect ','
    if chars.next()? != ',' {
        return None;
    }

    // 5. Parse x
    let x = parse_number(chars)?;

    // 6. Expect ','
    if chars.next()? != ',' {
        return None;
    }

    // 7. Parse y
    let y = parse_number(chars)?;

    // 8. Next character determines whether this is a Leaf or Container
    match chars.peek() {
        Some('{') => {
            chars.next(); // consume '{'
            let mut children = Vec::new();
            while let Some(&c) = chars.peek() {
                if c == '}' {
                    chars.next();
                    break;
                }
                if c == ',' {
                    chars.next();
                    continue;
                }
                if let Some(child) = parse_node(chars) {
                    children.push(child);
                } else {
                    chars.next();
                }
            }
            Some(LayoutNode::Container {
                split: LayoutSplit::Horizontal,
                width,
                height,
                x,
                y,
                children,
            })
        }
        Some('[') => {
            chars.next(); // consume '['
            let mut children = Vec::new();
            while let Some(&c) = chars.peek() {
                if c == ']' {
                    chars.next();
                    break;
                }
                if c == ',' {
                    chars.next();
                    continue;
                }
                if let Some(child) = parse_node(chars) {
                    children.push(child);
                } else {
                    chars.next();
                }
            }
            Some(LayoutNode::Container {
                split: LayoutSplit::Vertical,
                width,
                height,
                x,
                y,
                children,
            })
        }
        Some(',') => {
            chars.next(); // consume ','
            let pane_num = parse_number(chars)?;
            Some(LayoutNode::Leaf {
                width,
                height,
                x,
                y,
                pane_id: Some(PaneId(format!("%{pane_num}"))),
            })
        }
        _ => Some(LayoutNode::Leaf {
            width,
            height,
            x,
            y,
            pane_id: None,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_leaf_layout() {
        let layout_str = "bb62,204x50,0,0,1";
        let node = LayoutNode::parse(layout_str).expect("Failed to parse layout");
        match node {
            LayoutNode::Leaf {
                width,
                height,
                x,
                y,
                pane_id,
            } => {
                assert_eq!(width, 204);
                assert_eq!(height, 50);
                assert_eq!(x, 0);
                assert_eq!(y, 0);
                assert_eq!(pane_id, Some(PaneId::from("%1")));
            }
            _ => panic!("Expected leaf node"),
        }
    }

    #[test]
    fn test_parse_nested_layout() {
        let layout_str =
            "bb62,204x50,0,0{101x50,0,0,1,102x50,102,0[102x24,102,0,2,102x25,102,25,3]}";
        let node = LayoutNode::parse(layout_str).expect("Failed to parse nested layout");
        match node {
            LayoutNode::Container {
                split,
                children,
                width,
                height,
                ..
            } => {
                assert_eq!(split, LayoutSplit::Horizontal);
                assert_eq!(width, 204);
                assert_eq!(height, 50);
                assert_eq!(children.len(), 2);
            }
            _ => panic!("Expected container node"),
        }
    }
}
