use crate::exceptions::CustomErr;
use crate::db::column::ColumnValue;

#[derive(Debug, Clone)]
pub enum ValueExpr {
    Text(String),
    Integer(i64),
    Float(f64),
    Null,
}

impl PartialOrd for ValueExpr {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Follow SQLite's storage-class sort order: NULL < numeric < text.
        // The derived order would place Null last (largest), which breaks index
        // traversal because SQLite stores NULL keys first in the b-tree.
        fn rank(v: &ValueExpr) -> u8 {
            match v {
                ValueExpr::Null => 0,
                ValueExpr::Integer(_) | ValueExpr::Float(_) => 1,
                ValueExpr::Text(_) => 2,
            }
        }
        match (self, other) {
            (ValueExpr::Text(a), ValueExpr::Text(b)) => a.partial_cmp(b),
            (ValueExpr::Integer(a), ValueExpr::Integer(b)) => a.partial_cmp(b),
            (ValueExpr::Float(a), ValueExpr::Float(b)) => a.partial_cmp(b),
            (ValueExpr::Integer(a), ValueExpr::Float(b)) => (*a as f64).partial_cmp(b),
            (ValueExpr::Float(a), ValueExpr::Integer(b)) => a.partial_cmp(&(*b as f64)),
            _ => rank(self).partial_cmp(&rank(other)),
        }
    }
}

impl PartialEq for ValueExpr {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Text(a), Self::Text(b)) => a == b,
            (Self::Integer(a), Self::Integer(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a.to_bits() == b.to_bits(),
            (Self::Null, _) => false,
            (_, Self::Null) => false,
            _ => false,
        }
    }
}

impl TryFrom<&ColumnValue> for ValueExpr {
    type Error = CustomErr;

    fn try_from(value: &ColumnValue) -> Result<Self, Self::Error> {
        let value_expr = match value {
            ColumnValue::Int8(x) => Self::Integer(*x as i64),
            ColumnValue::Int16(x) => Self::Integer(*x as i64),
            ColumnValue::Int24(x) => Self::Integer(*x as i64),
            ColumnValue::Int48(x) => Self::Integer(*x),
            ColumnValue::Int64(x) => Self::Integer(*x),
            ColumnValue::Float64(x) => Self::Float(*x),
            ColumnValue::Text(s) => Self::Text(s.to_string()),
            ColumnValue::Null => Self::Null,
            _ => {
                return Err(CustomErr::Execution(
                    "Cannot be evaluated to scalar".to_string(),
                ));
            }
        };
        Ok(value_expr)
    }
}
