from fastapi import APIRouter, Depends, HTTPException, Query
from typing import List, Optional
from uuid import UUID
from pydantic import BaseModel, Field
import asyncpg
from datetime import datetime

from ..database import get_db
from ..auth import get_current_user

router = APIRouter(prefix="/api/labels", tags=["labels"])


class LabelCreate(BaseModel):
    name: str = Field(..., max_length=50)
    description: Optional[str] = None
    color: str = Field(default="#0969da", regex="^#[0-9a-fA-F]{6}$")
    background_color: Optional[str] = Field(None, regex="^#[0-9a-fA-F]{6}$")
    icon: Optional[str] = Field(None, max_length=50)


class LabelUpdate(BaseModel):
    name: Optional[str] = Field(None, max_length=50)
    description: Optional[str] = None
    color: Optional[str] = Field(None, regex="^#[0-9a-fA-F]{6}$")
    background_color: Optional[str] = Field(None, regex="^#[0-9a-fA-F]{6}$")
    icon: Optional[str] = Field(None, max_length=50)


class Label(BaseModel):
    id: UUID
    user_id: UUID
    name: str
    description: Optional[str]
    color: str
    background_color: Optional[str]
    icon: Optional[str]
    is_system: bool
    created_at: datetime
    updated_at: datetime
    document_count: Optional[int] = 0
    source_count: Optional[int] = 0


class LabelAssignment(BaseModel):
    label_ids: List[UUID]


@router.get("/", response_model=List[Label])
async def get_labels(
    include_counts: bool = Query(False, description="Include usage counts"),
    current_user: dict = Depends(get_current_user),
    db: asyncpg.Connection = Depends(get_db)
):
    """Get all labels available to the current user."""
    user_id = current_user["id"]
    
    if include_counts:
        query = """
            SELECT 
                l.id, l.user_id, l.name, l.description, l.color, 
                l.background_color, l.icon, l.is_system, l.created_at, l.updated_at,
                COUNT(DISTINCT dl.document_id) as document_count,
                COUNT(DISTINCT sl.source_id) as source_count
            FROM labels l
            LEFT JOIN document_labels dl ON l.id = dl.label_id
            LEFT JOIN source_labels sl ON l.id = sl.label_id
            WHERE l.user_id = $1 OR l.is_system = TRUE
            GROUP BY l.id
            ORDER BY l.name
        """
    else:
        query = """
            SELECT 
                id, user_id, name, description, color, 
                background_color, icon, is_system, created_at, updated_at,
                0 as document_count, 0 as source_count
            FROM labels
            WHERE user_id = $1 OR is_system = TRUE
            ORDER BY name
        """
    
    rows = await db.fetch(query, user_id)
    return [Label(**row) for row in rows]


@router.post("/", response_model=Label)
async def create_label(
    label: LabelCreate,
    current_user: dict = Depends(get_current_user),
    db: asyncpg.Connection = Depends(get_db)
):
    """Create a new label."""
    user_id = current_user["id"]
    
    try:
        row = await db.fetchrow(
            """
            INSERT INTO labels (user_id, name, description, color, background_color, icon)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            """,
            user_id, label.name, label.description, label.color, 
            label.background_color, label.icon
        )
        return Label(**row, document_count=0, source_count=0)
    except asyncpg.UniqueViolationError:
        raise HTTPException(status_code=400, detail="Label with this name already exists")


@router.get("/{label_id}", response_model=Label)
async def get_label(
    label_id: UUID,
    current_user: dict = Depends(get_current_user),
    db: asyncpg.Connection = Depends(get_db)
):
    """Get a specific label."""
    user_id = current_user["id"]
    
    row = await db.fetchrow(
        """
        SELECT 
            l.*,
            COUNT(DISTINCT dl.document_id) as document_count,
            COUNT(DISTINCT sl.source_id) as source_count
        FROM labels l
        LEFT JOIN document_labels dl ON l.id = dl.label_id
        LEFT JOIN source_labels sl ON l.id = sl.label_id
        WHERE l.id = $1 AND (l.user_id = $2 OR l.is_system = TRUE)
        GROUP BY l.id
        """,
        label_id, user_id
    )
    
    if not row:
        raise HTTPException(status_code=404, detail="Label not found")
    
    return Label(**row)


@router.put("/{label_id}", response_model=Label)
async def update_label(
    label_id: UUID,
    label: LabelUpdate,
    current_user: dict = Depends(get_current_user),
    db: asyncpg.Connection = Depends(get_db)
):
    """Update a label."""
    user_id = current_user["id"]
    
    # Check if label exists and user has permission
    existing = await db.fetchrow(
        "SELECT * FROM labels WHERE id = $1 AND user_id = $2 AND is_system = FALSE",
        label_id, user_id
    )
    
    if not existing:
        raise HTTPException(status_code=404, detail="Label not found or cannot be modified")
    
    # Build update query dynamically
    updates = []
    values = [label_id]
    param_index = 2
    
    if label.name is not None:
        updates.append(f"name = ${param_index}")
        values.append(label.name)
        param_index += 1
    
    if label.description is not None:
        updates.append(f"description = ${param_index}")
        values.append(label.description)
        param_index += 1
    
    if label.color is not None:
        updates.append(f"color = ${param_index}")
        values.append(label.color)
        param_index += 1
    
    if label.background_color is not None:
        updates.append(f"background_color = ${param_index}")
        values.append(label.background_color)
        param_index += 1
    
    if label.icon is not None:
        updates.append(f"icon = ${param_index}")
        values.append(label.icon)
        param_index += 1
    
    if not updates:
        return Label(**existing, document_count=0, source_count=0)
    
    updates.append("updated_at = CURRENT_TIMESTAMP")
    
    try:
        row = await db.fetchrow(
            f"""
            UPDATE labels 
            SET {', '.join(updates)}
            WHERE id = $1
            RETURNING *
            """,
            *values
        )
        return Label(**row, document_count=0, source_count=0)
    except asyncpg.UniqueViolationError:
        raise HTTPException(status_code=400, detail="Label with this name already exists")


@router.delete("/{label_id}")
async def delete_label(
    label_id: UUID,
    current_user: dict = Depends(get_current_user),
    db: asyncpg.Connection = Depends(get_db)
):
    """Delete a label."""
    user_id = current_user["id"]
    
    result = await db.execute(
        "DELETE FROM labels WHERE id = $1 AND user_id = $2 AND is_system = FALSE",
        label_id, user_id
    )
    
    if result == "DELETE 0":
        raise HTTPException(status_code=404, detail="Label not found or cannot be deleted")
    
    return {"message": "Label deleted successfully"}


@router.get("/documents/{document_id}", response_model=List[Label])
async def get_document_labels(
    document_id: UUID,
    current_user: dict = Depends(get_current_user),
    db: asyncpg.Connection = Depends(get_db)
):
    """Get all labels for a document."""
    user_id = current_user["id"]
    
    # Verify document ownership
    doc = await db.fetchrow(
        "SELECT id FROM documents WHERE id = $1 AND user_id = $2",
        document_id, user_id
    )
    
    if not doc:
        raise HTTPException(status_code=404, detail="Document not found")
    
    rows = await db.fetch(
        """
        SELECT l.*, 0 as document_count, 0 as source_count
        FROM labels l
        INNER JOIN document_labels dl ON l.id = dl.label_id
        WHERE dl.document_id = $1
        ORDER BY l.name
        """,
        document_id
    )
    
    return [Label(**row) for row in rows]


@router.put("/documents/{document_id}", response_model=List[Label])
async def update_document_labels(
    document_id: UUID,
    assignment: LabelAssignment,
    current_user: dict = Depends(get_current_user),
    db: asyncpg.Connection = Depends(get_db)
):
    """Replace all labels for a document."""
    user_id = current_user["id"]
    
    # Verify document ownership
    doc = await db.fetchrow(
        "SELECT id FROM documents WHERE id = $1 AND user_id = $2",
        document_id, user_id
    )
    
    if not doc:
        raise HTTPException(status_code=404, detail="Document not found")
    
    # Verify all labels exist and are accessible
    label_check = await db.fetch(
        """
        SELECT id FROM labels 
        WHERE id = ANY($1::uuid[]) AND (user_id = $2 OR is_system = TRUE)
        """,
        assignment.label_ids, user_id
    )
    
    if len(label_check) != len(assignment.label_ids):
        raise HTTPException(status_code=400, detail="One or more labels not found")
    
    async with db.transaction():
        # Remove existing labels
        await db.execute(
            "DELETE FROM document_labels WHERE document_id = $1",
            document_id
        )
        
        # Add new labels
        if assignment.label_ids:
            await db.executemany(
                """
                INSERT INTO document_labels (document_id, label_id, assigned_by)
                VALUES ($1, $2, $3)
                """,
                [(document_id, label_id, user_id) for label_id in assignment.label_ids]
            )
    
    # Return updated labels
    return await get_document_labels(document_id, current_user, db)


@router.post("/documents/{document_id}/labels/{label_id}")
async def add_document_label(
    document_id: UUID,
    label_id: UUID,
    current_user: dict = Depends(get_current_user),
    db: asyncpg.Connection = Depends(get_db)
):
    """Add a single label to a document."""
    user_id = current_user["id"]
    
    # Verify document ownership
    doc = await db.fetchrow(
        "SELECT id FROM documents WHERE id = $1 AND user_id = $2",
        document_id, user_id
    )
    
    if not doc:
        raise HTTPException(status_code=404, detail="Document not found")
    
    # Verify label exists and is accessible
    label = await db.fetchrow(
        "SELECT id FROM labels WHERE id = $1 AND (user_id = $2 OR is_system = TRUE)",
        label_id, user_id
    )
    
    if not label:
        raise HTTPException(status_code=404, detail="Label not found")
    
    try:
        await db.execute(
            """
            INSERT INTO document_labels (document_id, label_id, assigned_by)
            VALUES ($1, $2, $3)
            """,
            document_id, label_id, user_id
        )
        return {"message": "Label added successfully"}
    except asyncpg.UniqueViolationError:
        return {"message": "Label already assigned"}


@router.delete("/documents/{document_id}/labels/{label_id}")
async def remove_document_label(
    document_id: UUID,
    label_id: UUID,
    current_user: dict = Depends(get_current_user),
    db: asyncpg.Connection = Depends(get_db)
):
    """Remove a single label from a document."""
    user_id = current_user["id"]
    
    # Verify document ownership
    doc = await db.fetchrow(
        "SELECT id FROM documents WHERE id = $1 AND user_id = $2",
        document_id, user_id
    )
    
    if not doc:
        raise HTTPException(status_code=404, detail="Document not found")
    
    result = await db.execute(
        "DELETE FROM document_labels WHERE document_id = $1 AND label_id = $2",
        document_id, label_id
    )
    
    if result == "DELETE 0":
        raise HTTPException(status_code=404, detail="Label not found on document")
    
    return {"message": "Label removed successfully"}


@router.post("/bulk/documents", response_model=dict)
async def bulk_update_document_labels(
    document_ids: List[UUID],
    assignment: LabelAssignment,
    mode: str = Query("replace", regex="^(replace|add|remove)$"),
    current_user: dict = Depends(get_current_user),
    db: asyncpg.Connection = Depends(get_db)
):
    """Bulk update labels for multiple documents."""
    user_id = current_user["id"]
    
    # Verify document ownership
    docs = await db.fetch(
        "SELECT id FROM documents WHERE id = ANY($1::uuid[]) AND user_id = $2",
        document_ids, user_id
    )
    
    if len(docs) != len(document_ids):
        raise HTTPException(status_code=400, detail="One or more documents not found")
    
    # Verify labels
    if assignment.label_ids:
        label_check = await db.fetch(
            """
            SELECT id FROM labels 
            WHERE id = ANY($1::uuid[]) AND (user_id = $2 OR is_system = TRUE)
            """,
            assignment.label_ids, user_id
        )
        
        if len(label_check) != len(assignment.label_ids):
            raise HTTPException(status_code=400, detail="One or more labels not found")
    
    async with db.transaction():
        if mode == "replace":
            # Remove all existing labels
            await db.execute(
                "DELETE FROM document_labels WHERE document_id = ANY($1::uuid[])",
                document_ids
            )
            # Add new labels
            if assignment.label_ids:
                values = [
                    (doc_id, label_id, user_id) 
                    for doc_id in document_ids 
                    for label_id in assignment.label_ids
                ]
                await db.executemany(
                    """
                    INSERT INTO document_labels (document_id, label_id, assigned_by)
                    VALUES ($1, $2, $3)
                    ON CONFLICT DO NOTHING
                    """,
                    values
                )
        
        elif mode == "add":
            if assignment.label_ids:
                values = [
                    (doc_id, label_id, user_id) 
                    for doc_id in document_ids 
                    for label_id in assignment.label_ids
                ]
                await db.executemany(
                    """
                    INSERT INTO document_labels (document_id, label_id, assigned_by)
                    VALUES ($1, $2, $3)
                    ON CONFLICT DO NOTHING
                    """,
                    values
                )
        
        elif mode == "remove":
            if assignment.label_ids:
                await db.execute(
                    """
                    DELETE FROM document_labels 
                    WHERE document_id = ANY($1::uuid[]) 
                    AND label_id = ANY($2::uuid[])
                    """,
                    document_ids, assignment.label_ids
                )
    
    return {
        "message": f"Labels {mode}d successfully",
        "documents_updated": len(document_ids)
    }